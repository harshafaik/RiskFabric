import polars as pl
import xgboost as xgb
import duckdb
import numpy as np
import os
import pickle
from model_utils import load_model, get_model_features
from sklearn.calibration import CalibratedClassifierCV, calibration_curve
from sklearn.frozen import FrozenEstimator
from sklearn.metrics import (
    roc_auc_score,
    average_precision_score,
    brier_score_loss
)
from argparse import ArgumentParser
from ml_utils import split_by_timestamp, find_gold_snapshot
from mlflow_logging import setup_mlflow, log_metadata_artifacts
from mlflow_wrapper import CalibratedFraudModel


def calculate_ece(y_true, y_prob, n_bins=10):
    bin_edges = np.linspace(0, 1, n_bins + 1)
    ece = 0.0
    total_samples = len(y_prob)

    for i in range(n_bins):
        bin_lower = bin_edges[i]
        bin_upper = bin_edges[i+1]
        in_bin = (y_prob >= bin_lower) & (y_prob < bin_upper) if i < n_bins - 1 else (y_prob >= bin_lower) & (y_prob <= bin_upper)
        bin_count = np.sum(in_bin)

        if bin_count > 0:
            actual_frac = np.mean(y_true[in_bin])
            pred_mean = np.mean(y_prob[in_bin])
            bin_weight = bin_count / total_samples
            bin_error = abs(pred_mean - actual_frac)
            ece += bin_weight * bin_error

    return ece


def calibrate_and_evaluate():
    parser = ArgumentParser(description="Calibrate XGBoost model probabilities using Gold Parquet snapshots")
    parser.add_argument("--snapshot", type=str, default=None, help="Specific snapshot directory")
    args = parser.parse_args()

    gold_path = find_gold_snapshot(args.snapshot)
    print(f"📊 Loading Gold snapshot: {gold_path}")

    conn = duckdb.connect()
    query = f"SELECT * FROM '{gold_path}'"
    df = pl.from_arrow(conn.execute(query).arrow())
    conn.close()

    target_col = "is_fraud"
    feature_cols = get_model_features()

    feature_cols = [c for c in feature_cols if c in df.columns]
    missing = set(feature_cols) - set(df.columns)
    if missing:
        print(f"   ⚠️ Model expects features not in Gold: {missing}")

    string_cols = [c for c in feature_cols if df[c].dtype == pl.String]

    print("🧠 Loading Pre-trained Model...")
    base_model = load_model(enable_categorical=True)

    print("✂️ Splitting data chronologically into Calibration and Evaluation sets...")
    df_cal, df_eval = split_by_timestamp(df, test_size=0.5)

    X_cal = df_cal.select(feature_cols).to_pandas()
    y_cal = df_cal[target_col].to_numpy().flatten()
    X_eval = df_eval.select(feature_cols).to_pandas()
    y_eval = df_eval[target_col].to_numpy().flatten()

    for c in string_cols:
        X_cal[c] = X_cal[c].astype("category")
        X_eval[c] = X_eval[c].astype("category")

    print(f"   -> Calibration Set Size:  {len(y_cal):,}")
    print(f"   -> Evaluation Set Size:   {len(y_eval):,}")
    print(f"   -> Legitimate / Fraud:   {np.sum(y_eval==0):,} / {np.sum(y_eval==1):,}")

    print("\n⚖️ Fitting Platt Scaling Calibrator (Sigmoid)...")
    cal_platt = CalibratedClassifierCV(estimator=FrozenEstimator(base_model), method='sigmoid')
    cal_platt.fit(X_cal, y_cal)

    print("📈 Fitting Isotonic Regression Calibrator...")
    cal_isotonic = CalibratedClassifierCV(estimator=FrozenEstimator(base_model), method='isotonic')
    cal_isotonic.fit(X_cal, y_cal)

    print("\n🔮 Evaluating raw and calibrated models on held-out test data...")
    y_prob_raw = base_model.predict_proba(X_eval)[:, 1]
    y_prob_platt = cal_platt.predict_proba(X_eval)[:, 1]
    y_prob_isotonic = cal_isotonic.predict_proba(X_eval)[:, 1]

    results = {}
    models_to_eval = {
        'Raw Model (Uncalibrated)': y_prob_raw,
        'Platt Scaling (Sigmoid)': y_prob_platt,
        'Isotonic Regression': y_prob_isotonic
    }

    for name, probs in models_to_eval.items():
        results[name] = {
            'ROC-AUC': roc_auc_score(y_eval, probs),
            'PR-AUC': average_precision_score(y_eval, probs),
            'Brier Loss': brier_score_loss(y_eval, probs),
            'ECE': calculate_ece(y_eval, probs, n_bins=10),
        }

    print("\n" + "="*80)
    print("       POST-TRAINING CALIBRATION METRICS COMPARISON (EVALUATION SET)")
    print("="*80)
    print(f"{'Calibration Strategy':<30} | {'ROC-AUC':<10} | {'PR-AUC':<10} | {'Brier Loss':<12} | {'ECE'}")
    print("-" * 80)
    for name, metrics in results.items():
        print(f"{name:<30} | {metrics['ROC-AUC']:<10.5f} | {metrics['PR-AUC']:<10.5f} | {metrics['Brier Loss']:<12.6f} | {metrics['ECE']:.5f}")
    print("="*85)

    os.makedirs("models", exist_ok=True)

    with open("models/calibrated_fraud_model_platt.pkl", "wb") as f:
        pickle.dump(cal_platt, f)

    with open("models/calibrated_fraud_model_isotonic.pkl", "wb") as f:
        pickle.dump(cal_isotonic, f)

    print("\n💾 Calibrated models saved to models/")

    # MLflow logging
    try:
        import mlflow
        setup_mlflow()
        snapshot_tag = os.path.basename(os.path.dirname(gold_path))

        booster = base_model.get_booster()
        feature_names = list(booster.feature_names)
        feature_types = list(booster.feature_types)

        metadata_dir = log_metadata_artifacts(feature_names, feature_types, "isotonic")

        with mlflow.start_run(run_name=f"calibrate_{snapshot_tag}"):
            mlflow.log_params({
                "calibration_method": "isotonic",
                "calibration_split": 0.5,
                "snapshot": snapshot_tag,
                "n_cal_samples": len(y_cal),
                "n_eval_samples": len(y_eval),
            })

            metrics = {}
            for name, res in results.items():
                safe_name = name.lower().replace(" ", "_").replace("(", "").replace(")", "")
                mlflow.log_metric(f"{safe_name}.roc_auc", float(res['ROC-AUC']))
                mlflow.log_metric(f"{safe_name}.pr_auc", float(res['PR-AUC']))
                mlflow.log_metric(f"{safe_name}.brier_loss", float(res['Brier Loss']))
                mlflow.log_metric(f"{safe_name}.ece", float(res['ECE']))

            mlflow.set_tags({
                "script": "calibrate_model",
                "calibration_method": "isotonic",
                "snapshot": snapshot_tag,
            })

            model_artifacts = {
                "calibrator.pkl": "models/calibrated_fraud_model_isotonic.pkl",
                "feature_names.json": os.path.join(metadata_dir, "feature_names.json"),
                "feature_types.json": os.path.join(metadata_dir, "feature_types.json"),
                "calibration_method.txt": os.path.join(metadata_dir, "calibration_method.txt"),
            }

            mlflow.pyfunc.log_model(
                artifact_path="fraud_scorer",
                python_model=CalibratedFraudModel(),
                artifacts=model_artifacts,
                registered_model_name="RiskFabric-Fraud",
            )
            print(f"   📊 MLflow run: {mlflow.active_run().info.run_id}")
    except Exception as e:
        print(f"   ⚠️ MLflow logging failed (non-fatal): {e}")


if __name__ == "__main__":
    calibrate_and_evaluate()

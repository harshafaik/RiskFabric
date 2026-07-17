import os
import glob
import xgboost as xgb


def find_latest_model():
    explicit = os.environ.get("FRAUD_MODEL_PATH")
    if explicit and os.path.exists(explicit):
        return explicit

    if os.path.exists("models/fraud_model_v1.json"):
        return "models/fraud_model_v1.json"

    date_models = sorted(glob.glob("models/fraud_model_*.json"))
    if date_models:
        return date_models[-1]

    if explicit:
        raise FileNotFoundError(f"FRAUD_MODEL_PATH set but file not found: {explicit}")
    raise FileNotFoundError("No model found in models/. Train first with train_xgboost.py.")


def load_model(path=None, enable_categorical=False):
    if path is None:
        path = find_latest_model()

    if not os.path.exists(path):
        raise FileNotFoundError(f"Model not found: {path}")

    model = xgb.XGBClassifier(enable_categorical=enable_categorical)
    model.load_model(path)
    return model


def get_model_features(path=None):
    model = load_model(path, enable_categorical=True)
    return list(model.get_booster().feature_names)

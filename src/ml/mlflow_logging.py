import os
import json
import tempfile


def setup_mlflow(experiment_name="Fraud Detection Pipeline"):
    import mlflow

    os.environ.setdefault("MLFLOW_ALLOW_FILE_STORE", "true")

    tracking_uri = os.environ.get("MLFLOW_TRACKING_URI", "file:./mlruns")
    mlflow.set_tracking_uri(tracking_uri)
    mlflow.set_experiment(experiment_name)


def get_or_create_experiment(name="Fraud Detection Pipeline"):
    import mlflow

    experiment = mlflow.get_experiment_by_name(name)
    if experiment is None:
        experiment_id = mlflow.create_experiment(name)
    else:
        experiment_id = experiment.experiment_id
    return experiment_id


def log_metadata_artifacts(feature_names, feature_types, calibration_method):
    tmpdir = tempfile.mkdtemp()

    with open(os.path.join(tmpdir, "feature_names.json"), "w") as f:
        json.dump(feature_names, f)

    with open(os.path.join(tmpdir, "feature_types.json"), "w") as f:
        json.dump(feature_types, f)

    with open(os.path.join(tmpdir, "calibration_method.txt"), "w") as f:
        f.write(calibration_method)

    return tmpdir

import json
import os
import pickle

import numpy as np
import pandas as pd
import mlflow.pyfunc


class CalibratedFraudModel(mlflow.pyfunc.PythonModel):

    def load_context(self, context):
        with open(context.artifacts["calibrator.pkl"], "rb") as f:
            self.calibrator = pickle.load(f)

        with open(context.artifacts["feature_names.json"]) as f:
            self.feature_names = json.load(f)
        with open(context.artifacts["feature_types.json"]) as f:
            self.feature_types = json.load(f)

    def predict(self, context, model_input):
        df = model_input.copy()

        for feat in self.feature_names:
            if feat not in df.columns:
                df[feat] = 0.0
        df = df[self.feature_names]

        for name, ftype in zip(self.feature_names, self.feature_types):
            if ftype == "c":
                df[name] = df[name].astype("category")
            elif ftype == "float":
                df[name] = df[name].astype("float32")
            elif ftype == "int":
                df[name] = df[name].astype("int32")

        return self.calibrator.predict_proba(df)[:, 1]


class LocalScoringWrapper:
    """Unified .predict() interface for local model/calibrator fallback."""

    def __init__(self, sklearn_model_or_calibrator):
        self._model = sklearn_model_or_calibrator
        self._is_calibrated = hasattr(self._model, "calibrated_classifiers_")

        if self._is_calibrated:
            base_estimator = self._model.calibrated_classifiers_[0].estimator
            booster = base_estimator.get_booster()
        else:
            booster = self._model.get_booster()

        self.feature_names = list(booster.feature_names)
        self.feature_types = list(booster.feature_types)

    def predict(self, df):
        df = df.copy()

        for feat in self.feature_names:
            if feat not in df.columns:
                df[feat] = 0.0
        df = df[self.feature_names]

        for name, ftype in zip(self.feature_names, self.feature_types):
            if ftype == "c":
                df[name] = df[name].astype("category")
            elif ftype == "float":
                df[name] = df[name].astype("float32")
            elif ftype == "int":
                df[name] = df[name].astype("int32")

        return self._model.predict_proba(df)[:, 1]

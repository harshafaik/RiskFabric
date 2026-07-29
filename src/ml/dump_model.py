import xgboost as xgb
import json
import os
from model_utils import find_latest_model, load_model


def _decode_categories(enc_entry):
    """Decode an XGBoost cats.enc entry's byte values into category strings.

    enc_entry is a dict with:
      - 'values': list of ASCII ints (all category strings concatenated)
      - 'offsets': list of start indices in values marking category boundaries
    """
    values = enc_entry.get("values", [])
    offsets = enc_entry.get("offsets", [])
    if not offsets or not values:
        return []

    cats = []
    for i, start in enumerate(offsets):
        end = offsets[i + 1] if i + 1 < len(offsets) else len(values)
        cat_bytes = bytes(values[start:end])
        cats.append(cat_bytes.decode("ascii"))
    return cats


def dump():
    model = load_model()
    booster = model.get_booster()
    model_path = find_latest_model()
    feature_names = list(booster.feature_names)
    feature_types = list(booster.feature_types)

    print(f"Feature Names: {feature_names}")
    print(f"Feature Types: {feature_types}")

    # Parse categories from JSON using proper path traversal
    with open(model_path, "r") as f:
        data = json.load(f)

    cat_encodings = (
        data.get("learner", {})
        .get("gradient_booster", {})
        .get("model", {})
        .get("cats", {})
        .get("enc", [])
    )

    categorical_mappings = {}
    for i, (name, ftype) in enumerate(zip(feature_names, feature_types)):
        if ftype == "c" and i < len(cat_encodings):
            cats = _decode_categories(cat_encodings[i])
            if cats:
                categorical_mappings[name] = cats
                print(f"  Categorical [{name}]: {cats}")

    # Export schema.json
    schema = {
        "model_path": model_path,
        "n_features": len(feature_names),
        "features": [
            {"name": name, "type": ftype, "index": i}
            for i, (name, ftype) in enumerate(zip(feature_names, feature_types))
        ],
        "categorical_mappings": categorical_mappings,
    }

    os.makedirs("models", exist_ok=True)
    schema_path = "models/schema.json"
    with open(schema_path, "w") as f:
        json.dump(schema, f, indent=2)
    print(f"\n Schema exported to {schema_path}")
    print(f"   {len(feature_names)} features, {len(categorical_mappings)} categorical")


if __name__ == "__main__":
    dump()

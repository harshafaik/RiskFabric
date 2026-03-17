import xgboost as xgb
import json

MODEL_PATH = "models/fraud_model_v1.json"

def dump():
    model = xgb.XGBClassifier()
    model.load_model(MODEL_PATH)
    booster = model.get_booster()
    print(f"Feature Names: {booster.feature_names}")
    print(f"Feature Types: {booster.feature_types}")
    
    # Try to extract categories from the JSON directly
    with open(MODEL_PATH, 'r') as f:
        data = json.load(f)
        try:
            cats = data['learner']['gradient_booster']['model']['cats']
            # Cats encoding is in 'enc' which is a list of strings joined together
            # This is hard to parse manually.
            # However, we can see if there are strings in there.
            import re
            all_text = json.dumps(cats)
            found = re.findall(r'"([^"]+)"', all_text)
            print(f"Strings found in cats: {found[:50]}")
        except KeyError:
            print("Categories not found in JSON path.")

if __name__ == "__main__":
    dump()

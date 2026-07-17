import xgboost as xgb
import json
from model_utils import find_latest_model, load_model


def dump():
    model = load_model()
    booster = model.get_booster()
    model_path = find_latest_model()
    print(f"Feature Names: {booster.feature_names}")
    print(f"Feature Types: {booster.feature_types}")
    
    # Try to extract categories from the JSON directly
    with open(model_path, 'r') as f:
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

# sound effects curated by https://github.com/JstrisPlus

import requests

# this function parses the object and returns a list of candidate links
def parse_object(object: dict | list | str | bool | int | None) -> list[str]:
    if object is None or type(object) in (bool, int):
        return []

    # base case; try saving the file
    if type(object) == str:
        return [object]

    isDict = type(object) == dict
    objects = []
    for key in object:
        if isDict:
            objects += parse_object(object[key])
        else:
            objects += parse_object(key)
    return objects

def try_download(url: str):
    try:
        response = requests.get(url)
        if response.status_code == 200:
            with open(url.replace('/', '_'), 'wb') as f:
                f.write(response.content)
    except:
        pass

def main():
    jstris_plus_presets = "https://raw.githubusercontent.com/JstrisPlus/jstris-plus-assets/main/presets/soundPresets.json"
    response = requests.get(jstris_plus_presets)
    
    assert response.status_code == 200
    res = parse_object(response.json())
    for url in res:
        try_download(url)

if __name__ == "__main__":
    main()
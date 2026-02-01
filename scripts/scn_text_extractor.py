import json
import os
import sys


def parse(data: dict[str, object]) -> list[dict[str, str]]:
    if "scenes" not in data:
        return []

    outcome: list[dict[str, str]] = []

    for scene in data["scenes"]:
        scene: dict[str, list[str]]
        if "texts" in scene:
            texts: list[str] = scene["texts"]
            for text_data in texts:
                character: str = text_data[0] or "独白"
                text: str = text_data[2] or "<empty>"
                outcome.append({"character": character, "text": text})

    return outcome


def bulk_parse(input_dir: str, output_dir: str, character_filter: str | None = None) -> None:
    # create output dir
    os.makedirs(output_dir, exist_ok=True)
    for filename in os.listdir(input_dir):
        if not filename.endswith(".json"):
            continue
        file_path = os.path.join(input_dir, filename)
        with open(file_path, "r") as f:
            content: dict[str, object] = json.load(f)
        texts = parse(content)
        if character_filter is not None:
            texts = list(filter(lambda text: text["character"] == character_filter, texts))
        if len(texts) == 0:
            continue
        # write to json
        basename = filename.removesuffix(".json")
        out_json_path = os.path.join(output_dir, f"{basename}_texts.json")
        with open(out_json_path, "w") as f:
            json.dump(texts, f, indent=4, ensure_ascii=False)


def main() -> None:
    args = sys.argv[1:]
    if len(args) < 2:
        print("Usage: script.py <input_dir> <output_dir> [filter]")
        sys.exit(1)
    input_dir = args[0]
    output_dir = args[1]

    character_filter = None
    if len(args) == 3:
        character_filter = args[2]

    bulk_parse(input_dir, output_dir, character_filter)


if __name__ == "__main__":
    main()

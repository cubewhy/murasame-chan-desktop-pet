# Thanks https://gist.github.com/SnailShea/6b9f41157cb9ff6b417f7841e9b73aaa
import os
import sys

from PIL import Image

def ext_to_png(img: str):
    path, ext = os.path.splitext(img)
    new_img = f"{path}.png"
    return new_img


def batch_conv(input_dir: str, output_dir: str) -> None:
    for filename in os.listdir(input_dir):
        if not filename.endswith(".tlg"):
            continue
        tlg_file_path = os.path.join(input_dir, filename)
        png_file_path = os.path.join(output_dir, ext_to_png(filename))
        png = Image.open(tlg_file_path).convert("RGBA")
        # save png file
        png.save(png_file_path, "png")
        print(f"Converted file {filename}")


def main():
    args = sys.argv[1:]
    if len(args) != 2:
        print("Usage: script.py <input_dir> <output_dir>")
        sys.exit(1)
    input_dir = args[0]
    output_dir = args[1]
    batch_conv(input_dir, output_dir)


if __name__ == "__main__":
    main()

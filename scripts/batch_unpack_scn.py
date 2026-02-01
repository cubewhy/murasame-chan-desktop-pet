# Note: please modify the path of freemote/PsbDecompiler.exe
import os
import platform
import shutil
import subprocess
import sys


def exec_pe(executable: str, arguments: list[str]) -> int:
    if platform.system() == "Windows":
        command = [executable] + arguments
    else:
        command = ["wine", executable] + arguments

    return subprocess.call(command)


def unpack_scn(
    file_path: str, output_dir: str, temp_dir: str, decompiler_exec: str
) -> None:
    # create output dir
    os.makedirs(output_dir, exist_ok=True)
    # copy file to temp dir
    filename = os.path.basename(file_path)
    temp_file_path = os.path.join(temp_dir, filename)
    temp_file_path = os.path.abspath(shutil.copyfile(file_path, temp_file_path))

    # execute PsbDecompiler
    code = exec_pe(os.path.abspath(decompiler_exec), [temp_file_path])
    if code != 0:
        raise Exception(f"Failed to decompile {filename}")
    # delete temp file
    os.remove(temp_file_path)
    # move json to output_dir
    json_file_name = filename.removesuffix(".scn") + ".json"
    json_file_path = os.path.join(temp_dir, json_file_name)
    _ = shutil.move(json_file_path, os.path.join(output_dir, json_file_name))


def batch_process(
    input_dir: str,
    output_dir: str,
    temp_dir: str = "./tmp",
    psb_decompiler_exec: str = "./freemote/PsbDecompile.exe",
) -> None:
    # create temp dir
    # list input dir
    os.makedirs(temp_dir, exist_ok=True)
    for filename in os.listdir(input_dir):
        # ignore files with no .scn extension
        if not filename.endswith(".scn"):
            continue
        file_path = os.path.join(input_dir, filename)
        unpack_scn(file_path, output_dir, temp_dir, psb_decompiler_exec)


def main() -> None:
    # get the input folder
    args = sys.argv[1:]
    if len(args) < 2:
        print("Usage: script.py <input_dir> <output_dir>")
        sys.exit(1)
    input_dir = args[0]
    output_dir = args[1]

    batch_process(input_dir, output_dir)


if __name__ == "__main__":
    main()

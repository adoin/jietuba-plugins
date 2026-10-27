"""Build Windows preview plugins and package them as .jtbplugin ZIP files."""
import json
import pathlib
import subprocess
import zipfile

ROOT = pathlib.Path(__file__).resolve().parents[1]

def main():
    subprocess.run(["cargo", "build", "--release", "--locked"], cwd=ROOT, check=True)
    destination = ROOT / "dist"
    destination.mkdir(exist_ok=True)
    for manifest_path in sorted((ROOT / "plugins").glob("*/plugin.json")):
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        output = destination / f"{manifest['id']}-{manifest['version']}.jtbplugin"
        executable = ROOT / "target" / "release" / manifest["command"]
        with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_DEFLATED) as package:
            package.write(manifest_path, "plugin.json")
            package.write(executable, manifest["command"])
        print(output)

if __name__ == "__main__":
    main()

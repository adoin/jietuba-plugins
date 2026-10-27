"""Build Windows preview plugins and package them as .jtbplugin ZIP files."""
import json
import pathlib
import subprocess
import zipfile

ROOT = pathlib.Path(__file__).resolve().parents[1]

def main():
    subprocess.run(["cargo", "build", "--release", "--locked"], cwd=ROOT, check=True)
    metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--format-version", "1", "--locked"], cwd=ROOT))
    licenses = []
    for crate in metadata["packages"]:
        if not crate.get("source"):
            continue
        folder = pathlib.Path(crate["manifest_path"]).parent
        for license_path in sorted(folder.glob("LICENSE*")):
            if license_path.is_file():
                licenses.append((license_path, f"licenses/{crate['name']}-{crate['version']}/{license_path.name}"))
    destination = ROOT / "dist"
    destination.mkdir(exist_ok=True)
    for manifest_path in sorted((ROOT / "plugins").glob("*/plugin.json")):
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        output = destination / f"{manifest['id']}-{manifest['version']}.jtbplugin"
        executable = ROOT / "target" / "release" / manifest["command"]
        with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_DEFLATED, strict_timestamps=False) as package:
            package.write(manifest_path, "plugin.json")
            package.write(executable, manifest["command"])
            package.write(ROOT / "LICENSE", "licenses/jietuba-plugins-LICENSE")
            for source, target in licenses:
                package.write(source, target)
        print(output)

if __name__ == "__main__":
    main()

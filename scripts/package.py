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
                licenses.append((license_path, f"{crate['name']} {crate['version']} / {license_path.name}"))
    notices = "\n\n".join(f"===== {name} =====\n{path.read_text(encoding='utf-8', errors='replace')}" for path, name in licenses)
    destination = ROOT / "dist"
    destination.mkdir(exist_ok=True)
    for manifest_path in sorted((ROOT / "plugins").glob("*/plugin.json")):
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        output = destination / f"{manifest['id']}-{manifest['version']}.jtbplugin"
        with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_DEFLATED, strict_timestamps=False) as package:
            package.write(manifest_path, "plugin.json")
            if "command" in manifest:
                executable = ROOT / "target" / "release" / manifest["command"]
                package.write(executable, manifest["command"])
            package.write(ROOT / "LICENSE", "licenses/jietuba-plugins-LICENSE")
            package.writestr("licenses/THIRD_PARTY_LICENSES.txt", notices.encode("utf-8"))
        print(output)

if __name__ == "__main__":
    main()

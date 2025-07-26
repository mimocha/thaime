# thaime

***Thai Input Method Engine - "Thaime"***

A Latin-to-Thai typing library

## Usage

1. (Pre-requisite) Python3 & IBus must be installed on your system

Install IBus, Python IBus binding, and GObject Introspection bindings

**Ubuntu:**

```bash
sudo apt update
sudo apt install -y ibus ibus-gtk3 python3-gi gir1.2-ibus-1.0 libibus-1.0-dev
```

**Fedora:**

```bash
sudo dnf update
sudo dnf install -y ibus ibus-devel ibus-gtk3 gobject-introspection gobject-introspection-devel python3-gobject-base python3-gobject-devel
```

2. Copy `thaime.xml` to the IBus components directory

```sh
sudo cp python-engine/thaime.xml /usr/share/ibus/component/
```

3. Start / restart the IBus daemon

```sh
ibus-daemon -drx
```

4. Verify that Thaime is registered with IBus

```sh
ibus list-engine | grep thaime
```

Should output: `thaime - Thaime`

5. Make the `ibus-engine-thaime` script executable

```sh
chmod +x python-engine/ibus-engine-thaime-python
```

6. Set the input engine to Thaime via your toolbar GUI or:

```sh
ibus engine thaime
```

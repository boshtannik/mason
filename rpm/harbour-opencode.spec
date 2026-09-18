Name:       harbour-opencode
Summary:    Native opencode client for Sailfish OS
Version:    0.1
Release:    1
Group:      Qt/Qt
License:    MIT
URL:        https://opencode.ai/
Source0:    %{name}-%{version}.tar.bz2
Requires:   sailfishsilica-qt5 >= 0.10.9
Requires:   libsailfishapp
BuildRequires:  rust
BuildRequires:  cargo
BuildRequires:  pkgconfig(Qt5Core)
BuildRequires:  pkgconfig(Qt5Qml)
BuildRequires:  pkgconfig(Qt5Quick)
BuildRequires:  pkgconfig(sailfishapp)
BuildRequires:  desktop-file-utils

%description
Native client for the opencode AI coding agent. Connects to the local
opencode server over SSE and provides a Silica chat UI.

# - PREP -----------------------------------------------------------------------
%prep
%setup -q -n %{name}-%{version}

# - BUILD ----------------------------------------------------------------------
%build
cd opencode-client
export RUSTFLAGS="-Clink-arg=-Wl,-z,relro,-z,now -Ccodegen-units=1 -Clink-arg=-rdynamic"
export CARGO_INCREMENTAL=0
export QMAKE=/usr/lib64/qt5/bin/qmake
export SB2_RUST_TARGET_TRIPLE=aarch64-unknown-linux-gnu
export CC_aarch64_unknown_linux_gnu=aarch64-meego-linux-gnu-gcc
export CXX_aarch64_unknown_linux_gnu=aarch64-meego-linux-gnu-g++
export AR_aarch64_unknown_linux_gnu=aarch64-meego-linux-gnu-ar
cargo build --release -j 1

# whisper.cpp: статический бинарник whisper-cli (STT). Сборка без OpenBLAS/COREML.
cd ../third_party/whisper.cpp
make -j1 \
  UNAME_M=generic \
  CC=aarch64-meego-linux-gnu-gcc CXX=aarch64-meego-linux-gnu-g++ \
  AR=aarch64-meego-linux-gnu-ar \
  main || true
test -x main
ls -l main

# - INSTALL --------------------------------------------------------------------
%install
rm -rf %{buildroot}
install -Dm 755 opencode-client/target/aarch64-unknown-linux-gnu/release/harbour-opencode -t %{buildroot}%{_bindir}
install -Dm 755 third_party/whisper.cpp/main %{buildroot}%{_libexecdir}/%{name}/whisper-cli
install -Dm 644 opencode-client/assets/models.json -t %{buildroot}%{_datadir}/%{name}
install -Dm 644 harbour-opencode.png -t %{buildroot}%{_datadir}/icons/hicolor/86x86/apps
install -Dm 644 harbour-opencode.desktop -t %{buildroot}%{_datadir}/applications
install -d %{buildroot}%{_datadir}/%{name}
cp -r opencode-client/qml %{buildroot}%{_datadir}/%{name}/qml

desktop-file-install --delete-original    \
  --dir %{buildroot}%{_datadir}/applications    \
   %{buildroot}%{_datadir}/applications/*.desktop

# - CHECK ----------------------------------------------------------------------
%check
if nm -D opencode-client/target/aarch64-unknown-linux-gnu/release/harbour-opencode | grep " T main$" > /dev/null ; then
  echo "main symbol exists"
else
  echo "main symbol is missing"
  exit 1
fi

# - FILES ----------------------------------------------------------------------
%files
%defattr(-,root,root,-)
%{_bindir}/harbour-opencode
%{_libexecdir}/%{name}/whisper-cli
%{_datadir}/%{name}/qml
%{_datadir}/%{name}/models.json
%{_datadir}/applications/%{name}.desktop
%{_datadir}/icons/hicolor/86x86/apps/%{name}.png

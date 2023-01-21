with import <nixpkgs> { };
stdenv.mkDerivation {
  name = "cnx";
  buildInputs = [
    alsaLib
    cairo
    clang
    glib
    gobject-introspection
    libclang
    libllvm
    llvmPackages.libclang
    openssl
    pango
    pkg-config
    python3
    rust-bindgen
    rustfmt
    wirelesstools
    xorg.libxcb
    xorg.xcbutilwm
  ];

  shellHook = ''
    export LIBCLANG_PATH="${llvmPackages.libclang.lib}/lib";
  '';
}

with import <nixpkgs> {};
  stdenv.mkDerivation {
    name = "cnx";
    buildInputs = [pkg-config alsaLib gobject-introspection cairo glib pango xorg.libxcb python3Full
                  openssl wirelesstools libllvm clang libclang rust-bindgen llvmPackages.libclang
                  xorg.libxcb xorg.xcbutilwm];

    shellHook = ''
    export LIBCLANG_PATH="${llvmPackages.libclang.lib}/lib";
    '';
  }

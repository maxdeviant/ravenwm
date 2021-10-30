with import <nixpkgs> {}; with xlibs;

stdenv.mkDerivation {
  name = "ravenwm";

  buildInputs = [
    stdenv
    pkg-config
    libX11
  ];
}

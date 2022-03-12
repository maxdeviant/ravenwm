with import <nixpkgs> {}; with xorg;

stdenv.mkDerivation {
  name = "ravenwm";

  buildInputs = [
    stdenv
    pkg-config
    libX11
  ];
}

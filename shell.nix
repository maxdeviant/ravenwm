with import <nixpkgs> {};

stdenv.mkDerivation {
  name = "ravenwm";

  buildInputs = [
    stdenv
    pkg-config
  ];
}

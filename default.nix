{ pkgs ? import <nixpkgs> { } }:
pkgs.rustPlatform.buildRustPackage rec {
  pname = "task-timer";
  version = "";

  src = ./.;

  cargoHash = "sha256-OXlvmn4uBiloSmn+AMPC7M/+g0MPGi9/4G43A8463kE=";

  buildInputs = with pkgs;
    [
      openssl
      pkg-config
      wayland
      xorg.libX11
      libGL
      libxkbcommon
      wayland
      xorg.libX11
      xorg.libXcursor
      xorg.libXi
      xorg.libXrandr
    ];
  nativeBuildInputs = with pkgs; [ fontconfig makeWrapper ];
  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
  postInstall = ''
    wrapProgram "$out/bin/task-timer" --prefix LD_LIBRARY_PATH : "${LD_LIBRARY_PATH}"
  '';
}

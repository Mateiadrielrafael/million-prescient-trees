{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/release-23.05";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachSystem
      (with flake-utils.lib.system; [ x86_64-linux ])
      (system:
        let pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          devShell = pkgs.mkShell {
            nativeBuildInputs = with pkgs; [
              rustup
              cargo
              gnuplot
            ];

            LD_LIBRARY_PATH = with pkgs; lib.makeLibraryPath [
              libGL
              libxkbcommon
              wayland
              xorg.libX11
              xorg.libXcursor
              xorg.libXi
              xorg.libXrandr
            ];
          };
        });
}

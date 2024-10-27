{
  description = "teleport";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?rev=63dacb46bf939521bdc93981b4cbb7ecb58427a0";
    rust-overlay.url = "github:oxalica/rust-overlay?rev=29b1275740d9283467b8117499ec8cbb35250584";
    nix-filter.url = "github:numtide/nix-filter?rev=3342559a24e85fc164b295c3444e8a139924675b";
  };

  outputs = { self, nixpkgs, rust-overlay, nix-filter }:
    let
      allSystems = [
        "x86_64-linux" # 64-bit Intel/AMD Linux
      ];

      forAllSystems = f: nixpkgs.lib.genAttrs allSystems (system: f {
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            rust-overlay.overlays.default
            self.overlays.default
          ];
        };
      });
      filter = nix-filter.lib;
    in
    {
      overlays.default = final: prev: {
        rustToolchain = final.rust-bin.stable.latest.default;
        #rustToolchain = final.rust-bin.fromRustupToolchainFile ./rust-toolchain;
      };

      packages = forAllSystems ({ pkgs }: {
        default =
          let
            rustPlatform = pkgs.makeRustPlatform {
              cargo = pkgs.rustToolchain;
              rustc = pkgs.rustToolchain;
            };
          in
          rustPlatform.buildRustPackage {
            name = "teleport";
            version = "0.1.0";
            src = filter {
              root = ./.;
              include = [
                ./Cargo.toml
                ./Cargo.lock
                ./rust-toolchain
                ./src
                ./abi
                ./templates
              ];
              exclude = [
                ./src/bin/redeem.rs
              ];
            };
            doCheck = false;
            cargoLock = {
              lockFile = ./Cargo.lock;
              # NOTE for git deps, the outputhash must be specified
              #
              #outputHashes = {
              #  "alloy-0.4.2" = pkgs.lib.fakeSha256;
              #};
              #
              # OR set:
              #
              #allowBuiltinFetchGit = true;
              #
              # see https://github.com/NixOS/nixpkgs/blob/master/doc/languages-frameworks/rust.section.md#importing-a-cargolock-file-importing-a-cargolock-file
            };

            nativeBuildInputs = with pkgs; [
              pkg-config
            ];
            buildInputs = with pkgs; [
              openssl
            ];

          };
        });

      devShells = forAllSystems ({ pkgs }: {
        default = pkgs.mkShell {
          packages = (with pkgs; [
            rustToolchain
          ]);
          nativeBuildInputs = with pkgs; [
            pkg-config
          ];
          buildInputs = with pkgs; [
            diffoscope
            openssl
          ];
        };
      });

    };
}

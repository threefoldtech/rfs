{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-23.11";

    crane.url = "github:ipetkov/crane";
    crane.inputs.nixpkgs.follows = "nixpkgs";

    flake-utils.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, crane, flake-utils }: flake-utils.lib.eachSystem
    [
      flake-utils.lib.system.x86_64-linux
      flake-utils.lib.system.aarch64-linux
      flake-utils.lib.system.aarch64-darwin
    ] (system: let
      pkgs = nixpkgs.legacyPackages.${system};
      craneLib = crane.lib.${system};
    in {
      devShells.default = craneLib.devShell {
        packages = [
          pkgs.rust-analyzer
        ];

        RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
      };
      packages.default = craneLib.buildPackage {
        src = self;

        # 2024-03-07 failing test:
        # > thread 'test::pack_unpack' has overflowed its stack
        # > fatal runtime error: stack overflow
        # > error: test failed, to rerun pass `--lib`
        #
        # appearantly needs `RUST_MIN_STACK: 8388608` according to https://github.com/threefoldtech/rfs/blob/eae5186cc6b0f8704f3e4715d2e3644f1f3baa2c/.github/workflows/tests.yaml#L25C1-L25C34
        doCheck = false;

        cargoExtraArgs = "--bin rfs --features=build-binary";

        nativeBuildInputs = [
          pkgs.perl
          pkgs.pkg-config
        ];

        buildInputs = [
          pkgs.openssl
          pkgs.openssl.dev
        ];
      };
  });
}

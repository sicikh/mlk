{
  description = "MLK: the compiler, the editor, and the tools they need";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    # The Rust toolchains are the official rustup distributions of one exact version;
    # `stable` is what the repository builds with, `nightly` is only there for rustfmt.
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
    }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f system);

      lib = nixpkgs.lib;

      # The `wasm-bindgen` crate and the `wasm-bindgen-cli` that generates the glue for it
      # must be the same version. nixpkgs builds one version of the tool; `Cargo.toml`
      # pins the crate to it, and the two are asserted below to be the same.
      wasmBindgenVersion = "0.2.127";

      # The browser the editor's check drives. nixpkgs has an older release, so the
      # assets are fetched here, pinned by the digests GitHub publishes for the release.
      obscura = {
        version = "0.2.3";
        assets = {
          x86_64-linux = {
            url = "https://github.com/h4ckf0r0day/obscura/releases/download/v0.2.3/obscura-x86_64-linux.tar.gz";
            hash = "sha256-FTTR5t2vPQgOxAketB0KTYzAQqSLYH08QQ/BO0gqnuw=";
          };
          aarch64-linux = {
            url = "https://github.com/h4ckf0r0day/obscura/releases/download/v0.2.3/obscura-aarch64-linux.tar.gz";
            hash = "sha256-Xs+YC8owYCNqeobsftg9lD5lmO6HyqRtIDJdkLx1+Xk=";
          };
          x86_64-darwin = {
            url = "https://github.com/h4ckf0r0day/obscura/releases/download/v0.2.3/obscura-x86_64-macos.tar.gz";
            hash = "sha256-18SBIt68KtmySELfRFYIYNunZeqSiz9ja38FMiUkURY=";
          };
          aarch64-darwin = {
            url = "https://github.com/h4ckf0r0day/obscura/releases/download/v0.2.3/obscura-aarch64-macos.tar.gz";
            hash = "sha256-RWU8+tIm8cm0FWA6LtWUd/y9YzXHQjOM4TPAXeC90FY=";
          };
        };
      };

      # The two assertions that keep the versions in step. They are forced by `envFor`,
      # so a bump of the crate without the flake (or the other way around) fails loudly
      # at evaluation instead of at the first wasm build.
      assertWasmBindgen =
        pkgs:
        if pkgs.wasm-bindgen-cli.version != wasmBindgenVersion then
          throw "flake.nix: nixpkgs packages wasm-bindgen-cli ${pkgs.wasm-bindgen-cli.version}, but this repository pins ${wasmBindgenVersion}; update flake.nix, Cargo.toml, and Cargo.lock together"
        else if
          !(lib.hasInfix ''
            name = "wasm-bindgen"
            version = "${wasmBindgenVersion}"
          '' (builtins.readFile ./Cargo.lock))
        then
          throw "flake.nix: Cargo.lock does not pin wasm-bindgen ${wasmBindgenVersion}; update Cargo.toml, Cargo.lock, and flake.nix together"
        else
          true;

      obscuraFor =
        pkgs:
        let
          system = pkgs.stdenv.hostPlatform.system;
          asset = obscura.assets.${system} or (throw "obscura ${obscura.version}: no release for ${system}");
        in
        pkgs.stdenvNoCC.mkDerivation {
          pname = "obscura";
          version = obscura.version;

          src = pkgs.fetchurl {
            inherit (asset) url hash;
          };

          # The archive holds the browser and the worker it runs beside it; both are
          # installed into `bin`, which is the layout the release ships and the CI
          # unpacked by hand before this derivation existed.
          sourceRoot = ".";
          dontConfigure = true;
          dontBuild = true;
          dontFixup = true;
          installPhase = ''
            mkdir -p $out/bin
            cp -R . $out/bin/
          '';

          meta = {
            description = "A headless browser for driving the MLK editor";
            homepage = "https://github.com/h4ckf0r0day/obscura";
            license = lib.licenses.asl20;
            platforms = lib.attrNames obscura.assets;
            sourceProvenance = [ lib.sourceTypes.binaryNativeCode ];
            mainProgram = "obscura";
          };
        };

      envFor =
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlays.default ];
          };

          toolchain = builtins.fromTOML (builtins.readFile ./rust-toolchain.toml);
          channel = toolchain.toolchain.channel;

          # The toolchain the repository builds with; the components and targets come
          # from `rust-toolchain.toml`, so a laptop and CI agree.
          rust = pkgs.rust-bin.stable.${channel}.minimal.override {
            extensions = (toolchain.toolchain.components or [ ]) ++ [
              "rust-analyzer"
              "rust-src"
            ];
            targets = toolchain.toolchain.targets or [ ];
          };

          # `just format` and `just gen-all` format with a nightly rustfmt, because
          # rustfmt.toml turns on unstable options. The date is the one the generated
          # files were last formatted with; bump it with the toolchain.
          rustNightly = pkgs.rust-bin.nightly."2026-09-30".minimal.override {
            extensions = [ "rustfmt" ];
          };

          # A `rustfmt` that is the pinned nightly one and nothing else. The stable toolchain
          # has no rustfmt; `cargo fmt` reads `RUSTFMT`, and xtask runs `rustfmt --version`.
          rustfmtNightly = pkgs.runCommand "rustfmt-nightly" { } ''
            mkdir -p $out/bin
            ln -s ${rustNightly}/bin/rustfmt $out/bin/rustfmt
            ln -s ${rustNightly}/bin/cargo-fmt $out/bin/cargo-fmt
          '';

          obscuraPkg = obscuraFor pkgs;
        in
        assert assertWasmBindgen pkgs;
        {
          inherit
            pkgs
            rust
            rustfmtNightly
            obscuraPkg
            ;
        };
    in
    {
      devShells = forAllSystems (
        system:
        let
          inherit (envFor system)
            pkgs
            rust
            rustfmtNightly
            obscuraPkg
            ;
        in
        {
          default = pkgs.mkShell {
            name = "mlk";

            packages = [
              rust
              rustfmtNightly
              pkgs.just
              pkgs.cargo-insta
              pkgs.wasm-bindgen-cli
              pkgs.wasm-tools
              pkgs.wasmtime
              pkgs.nodejs_26
              pkgs.corepack
              obscuraPkg
            ];

            shellHook = ''
              # `just format` and `just gen-all` format with a nightly rustfmt: `cargo fmt`
              # reads `RUSTFMT`, and xtask checks that `rustfmt --version` says nightly.
              # `rustfmtNightly` provides the one `rustfmt` the shell has.
              export RUSTFMT="${rustfmtNightly}/bin/rustfmt"

              # pnpm is the one corepack pins in `package.json`; it is downloaded on
              # first use, with the integrity hash from that field.
              export COREPACK_HOME="''${XDG_CACHE_HOME:-$HOME/.cache}/mlk/corepack"
              mkdir -p "$COREPACK_HOME/pnpm"
              corepack enable --install-directory "$COREPACK_HOME/pnpm" pnpm >/dev/null 2>&1 || true
              export PATH="$COREPACK_HOME/pnpm:$PATH"

              echo "mlk dev shell"
              echo "  rust    $(rustc --version)"
              echo "  fmt     $(rustfmt --version)"
              echo "  node    $(node --version)"
              echo "  wasm    $(wasm-bindgen --version), $(wasm-tools --version)"
              echo "  obscura $(obscura --version)"
              echo
              echo "run \`pnpm install\` once, then \`just verify\`."
              echo "\`just install-tools\` is for setups without this shell."
            '';
          };
        }
      );

      packages = forAllSystems (system: {
        obscura = (envFor system).obscuraPkg;
      });

      apps = forAllSystems (system: {
        obscura = {
          type = "app";
          program = "${self.packages.${system}.obscura}/bin/obscura";
          meta.description = "The browser the editor's check drives";
        };
      });
    };
}

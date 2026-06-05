{
  description = "Symbolica dev shell and workflow commands";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { nixpkgs, ... }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      eachSystem = f: nixpkgs.lib.genAttrs systems (system: f (import nixpkgs { inherit system; }));
      loadLicense = ''
        # shellcheck disable=SC1091
        [ ! -f .envrc ] || source ./.envrc >/dev/null 2>&1 || true
      '';
    in {
      devShells = eachSystem (pkgs:
        let
          libs = [ pkgs.gmp pkgs.mpfr ];
          tools = [ pkgs.binaryen pkgs.cargo pkgs.git pkgs.gnum4 pkgs.lld pkgs.maturin pkgs.pkg-config pkgs.python312 pkgs.rustc pkgs.uv ]
            ++ libs ++ pkgs.lib.optionals pkgs.stdenv.isLinux [ pkgs.patchelf ];
        in {
          default = pkgs.mkShell {
            packages = tools;
            shellHook = ''
              export GMP_MPFR_SYS_USE_SYSTEM_LIBS=1
              export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath libs}:''${LD_LIBRARY_PATH:-}"
            '' + loadLicense;
          };
        });

      apps = eachSystem (pkgs:
        let
          libs = [ pkgs.gmp pkgs.mpfr ];
          path = [ pkgs.binaryen pkgs.cargo pkgs.coreutils pkgs.git pkgs.gnum4 pkgs.lld pkgs.maturin pkgs.pkg-config pkgs.python312 pkgs.rustc pkgs.tailscale pkgs.typst pkgs.uv ]
            ++ libs ++ pkgs.lib.optionals pkgs.stdenv.isLinux [ pkgs.patchelf ];
          env = ''
            export GMP_MPFR_SYS_USE_SYSTEM_LIBS=1
            export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath libs}:''${LD_LIBRARY_PATH:-}"
          '' + loadLicense;
          app = name: text: {
            type = "app";
            program = "${pkgs.writeShellApplication { inherit name; runtimeInputs = path; text = env + text; }}/bin/${name}";
          };
          wheelSetup = ''
            shopt -s nullglob
            wheels=(dist/symbolica-*.whl)
            if [ "''${#wheels[@]}" -eq 0 ]; then
              maturin build --out dist
              wheels=(dist/symbolica-*.whl)
            fi
            for wheel in "''${wheels[@]}"; do :; done
          '';
        in rec {
          default = build;
          build = app "symbolica-build" ''if [ $# = 0 ]; then cargo build --workspace; else cargo build "$@"; fi'';
          test = app "symbolica-test" ''if [ $# = 0 ]; then cargo test --workspace -- --test-threads=1; else cargo test "$@"; fi'';
          wheel = app "symbolica-wheel" ''maturin build --out dist "$@"'';
          typst-wasm = app "symbolica-typst-wasm" ''
            target=wasm32-unknown-unknown
            export RUSTFLAGS="''${RUSTFLAGS:-} --cfg getrandom_backend=\"custom\""
            cargo build --package symbolica-typst-plugin --profile wasm-release --target "$target" "$@"
            wasm-opt -Oz --quiet --enable-bulk-memory --enable-bulk-memory-opt --enable-nontrapping-float-to-int --strip-debug --strip-producers -o typst/symbolica/symbolica.wasm "target/$target/wasm-release/symbolica_typst_plugin.wasm"
            ls -lh typst/symbolica/symbolica.wasm
          '';
          typst-manual = app "symbolica-typst-manual" ''
            out="''${SYMBOLICA_MANUAL_OUT:-dist/symbolica-manual.pdf}"
            if [ "$#" -gt 0 ]; then
              out="$1"
              shift
            fi
            mkdir -p "$(dirname "$out")"
            typst compile --root typst/symbolica typst/symbolica/manual.typ "$out" "$@"
            ls -lh "$out"
          '';
          marimo = app "symbolica-marimo" (wheelSetup + ''
            exec uv run --no-project --python ${pkgs.python312}/bin/python3 --with "$wheel" --with marimo marimo edit "$@"
          '');
          marimo-tailscale = app "symbolica-marimo-tailscale" (wheelSetup + ''
            host="''${SYMBOLICA_MARIMO_HOST:-}"
            if [ -z "$host" ]; then
              host="$(tailscale ip -4 | head -n1)"
            fi
            if [ -z "$host" ]; then
              echo "Could not determine Tailscale IPv4; set SYMBOLICA_MARIMO_HOST." >&2
              exit 1
            fi
            port="''${SYMBOLICA_MARIMO_PORT:-2718}"
            echo "Starting marimo at http://$host:$port"
            exec uv run --no-project --python ${pkgs.python312}/bin/python3 --with "$wheel" --with marimo marimo edit --headless --host "$host" --port "$port" "$@"
          '');
        });
    };
}

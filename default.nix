{ depot ? { }, pkgs ? (import <nixpkgs> { }).pkgs, ... }:

pkgs.rustPlatform.buildRustPackage {
  name = "planet-mars";
  src = depot.third_party.gitignoreSource or (x: x) ./.;
  cargoLock.lockFile = ./Cargo.lock;

  nativeBuildInputs = [ pkgs.pkg-config ];
  buildInputs = [ pkgs.openssl ];

  passthru = {
    # planet-mars is mirrored to Github.
    meta.ci.extraSteps =
      if depot == { } then { }
      else {
        github =
          depot.tools.releases.filteredGitPush {
            filter = ":/web/planet-mars";
            remote = "git@github.com:thkoch2001/planet-mars.git";
            ref = "refs/heads/master";
          };
      };
    # templates need to be available for configuration
    templates = ./templates;
  };
}

# Contribution guidelines

First off, thank you for considering contributing to h2kv.

If your contribution is not straightforward, please first discuss the change you
wish to make by creating a new issue before making the change.

## Code quality

The maintainers of this project evaluate potential contributions based on specific
tenets in order to prevent abuse of the open source model of software development.
This is in agreement with the Unix philosophy of doing one thing well.

* usage over contributions
* simplicity over circumspection
* tidiness over efficiency

In summary, the needs of the project are best served by code contributions which not
only execute well, but also read well.

## Reporting issues

Before reporting an issue on the
[issue tracker](https://github.com/guapodero/h2kv/issues),
please check that it has not already been reported by searching for some related
keywords.

## Pull requests

Try to do one pull request per change.

### Updating the changelog

Update the changes you have made in
[CHANGELOG](https://github.com/guapodero/h2kv/blob/main/CHANGELOG.md)
file under the **Unreleased** section.

Add the changes of your pull request to one of the following subsections,
depending on the types of changes defined by
[Keep a changelog](https://keepachangelog.com/en/1.0.0/):

- `Added` for new features.
- `Changed` for changes in existing functionality.
- `Deprecated` for soon-to-be removed features.
- `Removed` for now removed features.
- `Fixed` for any bug fixes.
- `Security` in case of vulnerabilities.

If the required subsection does not exist yet under **Unreleased**, create it!

## Developing

This project relies on a set of integration tests to encourage refactoring and
prevent feature regression. These tests require system-level dependencies
which are provided by the Nix package manager.

```shell
git clone https://github.com/guapodero/h2kv
cd h2kv
cargo build
cargo xtask test
```

# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Richard Majewski - Varanid Works

Name:           podci
Version:        0.1.0
Release:        1%{?dist}
Summary:        podCI: Podman Continuous Integration runner (local-first CI parity)

License:        MIT OR Apache-2.0
URL:            https://github.com/UglyEgg/podCI


Source0:        %{url}/archive/refs/tags/v%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  rust-packaging
BuildRequires:  bash-completion
BuildRequires:  zsh
BuildRequires:  fish
Requires:       podman

%description
A local-first CI parity runner for Podman with deterministic caching.

%prep
%autosetup

%build
# Generate man page + shell completions into ./dist/
%{__cargo} run -p podci --bin podci-assets --features gen-assets --locked -- gen
%cargo_build

%check
%cargo_test

%install
%cargo_install
install -Dm644 dist/podci.1 %{buildroot}%{_mandir}/man1/podci.1
install -Dm644 dist/completions/podci.bash %{buildroot}%{_datadir}/bash-completion/completions/podci
install -Dm644 dist/completions/_podci %{buildroot}%{_datadir}/zsh/site-functions/_podci
install -Dm644 dist/completions/podci.fish %{buildroot}%{_datadir}/fish/vendor_completions.d/podci.fish
mkdir -p %{buildroot}%{_datadir}/podci/templates
cp -a templates/* %{buildroot}%{_datadir}/podci/templates/

%files
%license LICENSE*
%doc README.md
%{_bindir}/podci
%{_mandir}/man1/podci.1*
%{_datadir}/bash-completion/completions/podci
%{_datadir}/zsh/site-functions/_podci
%{_datadir}/fish/vendor_completions.d/podci.fish
%{_datadir}/podci/templates

%changelog
* Wed Feb 18 2026 Your Name <you@example.com> - 0.1.0-1
- Initial release

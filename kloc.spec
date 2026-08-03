%global _cargo_build_flags --release --locked

Name:    kloc
Version: 0.2.0
Release: 1%{?dist}
Summary: Count lines of code and code complexity via universal AST parsing

License: GPL-2.0-only
URL:     https://github.com/mgajda/kloc
Source0: https://github.com/mgajda/kloc/archive/v%{version}/kloc-%{version}.tar.gz

BuildRequires: rust >= 1.89, cargo, gcc-c++, python3-devel
Requires:      libgcc

%description
kloc uses tree-sitter for AST-based analysis, counting source lines of code
(SLOC), comments, blank lines, and code complexity metrics across many
programming languages. Each language is a Cargo feature gate.

%prep
%autosetup

%build
cargo build %{_cargo_build_flags}

%install
mkdir -p %{buildroot}%{_bindir}
install -m 0755 target/release/kloc %{buildroot}%{_bindir}/

%check
cargo test --release --locked

%files
%{_bindir}/kloc
%license LICENSE
%doc README.md

%changelog
* Wed Jul 30 2026 Michal J. Gajda <mjgajda@migamake.com>
- Initial package

.PHONY: mbuild mbuild-release clean test test-sdk

# Сборка RPM под Sailfish (Docker sailo-rs + mb2). Debug по умолчанию (быстро),
# для релиза — `make mbuild-release` (с оптимизациями).
mbuild:
	sg docker -c "./mbuild.sh"

mbuild-release:
	sg docker -c "./mbuild.sh --release"

# Тесты чистого протокола на хосте (без SFOS-тулчейна) — быстро.
test:
	cargo test --manifest-path opencode-proto/Cargo.toml

# Тесты библиотеки opencode-client (линкует Qt/SFOS) — в SDK через sb2+qemu.
test-sdk:
	sg docker -c "./scripts/test-sdk.sh"

clean:
	rm -rf opencode-client/target opencode-proto/target RPMS

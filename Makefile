.PHONY: mbuild mbuild-release clean

# Сборка RPM под Sailfish (Docker sailo-rs + mb2). Debug по умолчанию (быстро),
# для релиза — `make mbuild-release` (с оптимизациями).
mbuild:
	sg docker -c "./mbuild.sh"

mbuild-release:
	sg docker -c "./mbuild.sh --release"

clean:
	rm -rf opencode-client/target RPMS

.PHONY: mbuild clean

# Сборка RPM под Sailfish (Docker sailo-rs + mb2).
mbuild:
	sg docker -c "./mbuild.sh"

clean:
	rm -rf opencode-client/target RPMS

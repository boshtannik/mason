.PHONY: build build-arm build-arm-release check check-all deploy deploy-release

# Сборка под aarch64 (телефон)
build-arm:
	scripts/build.sh -a

build-arm-release:
	scripts/build.sh -ar

# Все серверные проверки (guard_smoke + full_cycle)
check:
	scripts/check.sh

# Полный набор проверок (включая голос и аудио)
check-all:
	scripts/check.sh --voice --audio

# Деплой debug-бинаря на телефон
deploy:
	scripts/deploy.sh

deploy-release:
	scripts/deploy.sh -r
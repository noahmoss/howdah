.PHONY: test test-integration

test:
	cargo test --workspace

test-integration:
	@set -eu; \
	trap 'docker compose down' EXIT; \
	trap 'exit 130' INT; \
	trap 'exit 143' TERM; \
	docker compose up --wait --wait-timeout 60; \
	HOWDAH_TEST_DATABASE_URL='postgresql://howdah:howdah@127.0.0.1:55432/howdah_test' \
	  cargo test --workspace -- --ignored

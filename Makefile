.PHONY: install test e2e release-check package-release homebrew-formula

install:
	cargo install --locked --path .

test:
	cargo test

e2e:
	./scripts/e2e-macos.sh

release-check:
	cargo fmt --check
	cargo test
	./scripts/e2e-macos.sh

package-release:
	./scripts/package-release.sh

homebrew-formula:
	./scripts/generate-homebrew-formula.sh \
		--version "$(VERSION)" \
		--homepage "$(HOMEPAGE)" \
		--arm-url "$(ARM_URL)" \
		--arm-sha256 "$(ARM_SHA256)" \
		--intel-url "$(INTEL_URL)" \
		--intel-sha256 "$(INTEL_SHA256)" \
		$(if $(OUTPUT),--output "$(OUTPUT)",)

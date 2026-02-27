.PHONY: dev backend frontend test test-unit test-e2e

# Run both backend and frontend dev servers concurrently
dev:
	$(MAKE) backend & $(MAKE) frontend & wait

# Backend API server on port 3000
backend:
	cargo run -p foxhole-backend

# Dioxus dev server (proxies /graphql and /assets to backend)
frontend:
	cd crates/frontend && dx serve

# Run all tests
test: test-unit test-e2e

# Rust unit tests
test-unit:
	cargo test --workspace

# Playwright end-to-end tests (starts backend+frontend automatically)
test-e2e:
	npx playwright test

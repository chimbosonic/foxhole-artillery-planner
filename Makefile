.PHONY: dev backend frontend

# Run both backend and frontend dev servers concurrently
dev:
	$(MAKE) backend & $(MAKE) frontend & wait

# Backend API server on port 3000
backend:
	cargo run -p foxhole-backend

# Dioxus dev server (proxies /graphql and /assets to backend)
frontend:
	cd crates/frontend && dx serve

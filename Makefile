include .env
export

types:
	cargo run --bin schema 
	npm run types --prefix ./js

build: 
	- docker run --rm -v "$(shell pwd)":/code --platform linux/amd64 --mount type=volume,source="$(shell basename `pwd`)_cache",target=/code/target --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry cosmwasm/rust-optimizer:0.12.9
	- npm install --prefix ./js
upload:
	- npm run upload --prefix ./js -- --auto
instantiate:
	- npm run instantiate --prefix ./js -- --auto
migrate:
	- npm run migrate --prefix ./js
deploy: 
	- npm run deploy --prefix ./js
update:
	- npm run update --prefix ./js
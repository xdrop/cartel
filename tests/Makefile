PYTHON = poetry run python
CARGO = cargo

.PHONY = run format help

.DEFAULT_GOAL = help

help:
	@echo "---------------HELP-----------------"
	@echo "To run the tests type make run"
	@echo "To format the tests type make format"
	@echo "------------------------------------"

install:
	poetry install

run: install
	cd ../ && ${CARGO} build --all
	cd execshim && ${CARGO} build --release
	${PYTHON} -m pytest -vvx runtime

all: install
	cd ../ && ${CARGO} build --all
	cd execshim && ${CARGO} build --release
	${PYTHON} -m pytest --runslow -vv runtime

format: install
	${PYTHON} -m black .
	${PYTHON} -m isort .
	${PYTHON} -m flake8 .

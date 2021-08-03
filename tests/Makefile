PYTHON = poetry run python
CARGO = cargo

.PHONY = run format help

.DEFAULT_GOAL = help

help:
	@echo "---------------HELP-----------------"
	@echo "To run the tests type make run"
	@echo "To format the tests type make format"
	@echo "------------------------------------"

run:
	cd execshim && ${CARGO} build --release
	${PYTHON} -m pytest -vv
	
format:
	${PYTHON} -m black .
	${PYTHON} -m isort .
	${PYTHON} -m flake8 .

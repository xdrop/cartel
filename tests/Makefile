PYTHON = poetry run python

.PHONY = run format help

.DEFAULT_GOAL = help

help:
	@echo "---------------HELP-----------------"
	@echo "To run the tests type make run"
	@echo "To format the tests type make format"
	@echo "------------------------------------"

run:
	${PYTHON} -m pytest -vv
	
format:
	${PYTHON} -m black .
	${PYTHON} -m isort .

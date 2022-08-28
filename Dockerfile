FROM python:3.10-slim

RUN mkdir /listenssh
WORKDIR /listenssh
RUN pip install --upgrade pip
COPY requirements.txt /listenssh/

RUN pip install -r requirements.txt
COPY . /listenssh/

CMD ["python", "main.py"]
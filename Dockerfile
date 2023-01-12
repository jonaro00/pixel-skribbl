FROM ubuntu

WORKDIR /app
COPY ./target/release/backend .
COPY ./frontend/dist ./frontend/dist

CMD ["./backend"]

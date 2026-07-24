FROM alpine:3.19

RUN apk add --no-cache ca-certificates

COPY eci /usr/local/bin/eci
RUN chmod +x /usr/local/bin/eci

ENTRYPOINT ["eci"]

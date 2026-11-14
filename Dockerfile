# Aysha's Music Player — Docker image for free-tier deployment
# Build: docker build -t aysha-music .
# Run:   docker run -p 5050:5050 aysha-music

FROM python:3.11-slim

WORKDIR /app

# Install yt-dlp + runtime deps (no mpv/bluez on free tiers — graceful fallback)
RUN apt-get update && apt-get install -y \
    curl ca-certificates ffmpeg \
    && rm -rf /var/lib/apt/lists/*

# Install yt-dlp
RUN curl -L https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -o /usr/local/bin/yt-dlp \
    && chmod a+rx /usr/local/bin/yt-dlp

# Copy server
COPY server.py /app/server.py

EXPOSE 5050

ENV PYTHONUNBUFFERED=1

CMD ["python3", "server.py"]

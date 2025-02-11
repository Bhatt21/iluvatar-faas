# Python runtime configuration.
BASE_IMAGE = "docker.io/sakbhatt/iluvatar_base_image"
SERVER_COMMAND = "gunicorn -w 1 server:app"

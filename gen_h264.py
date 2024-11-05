import socket


def stream_file_to_server(file_path, server_address):
    """
    Streams the contents of a file to a TCP server.

    Parameters:
    file_path (str): Path to the file to be streamed.
    server_address (tuple): (host, port) tuple of the server to connect to.
    """
    # Create a TCP socket
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)

    try:
        # Connect to the server
        sock.connect(server_address)

        # Open the file and read its contents
        with open(file_path, "rb") as file:
            # Send the file data in chunks
            while True:
                chunk = file.read(1024)
                if not chunk:
                    break
                sock.sendall(chunk)

    finally:
        # Clean up the socket
        sock.close()


# Example usage
stream_file_to_server("data/test_stream.h264", ("127.0.0.1", 6969))

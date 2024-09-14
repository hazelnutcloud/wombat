# Wombat Discord Bot

Wombat is a Discord bot that allows you to interface with services running locally on your machine from anywhere through Discord. It provides a convenient way to make HTTP requests to your local services using Discord commands.

## Features

- Make HTTP requests (GET, POST, PUT, DELETE, PATCH) to local services
- Set custom headers for requests
- Send JSON payloads in request bodies
- Parse JSON responses with JSONPath
- Formatted output for both text and JSON responses

## Setup

1. Clone the repository
2. Create a `.env` file in the root directory and add your Discord bot token:
   ```
   DISCORD_BOT_TOKEN=your_token_here
   ```
3. Install dependencies: `cargo build`
4. Run the bot: `cargo run`

## Usage

The bot uses the prefix `~` for commands. The main command is `fetch`:

```
~fetch <url> [method] [headers] [body] [json_path]
```

- `url`: The URL to send the request to
- `method`: (Optional) The HTTP method (GET, POST, PUT, DELETE, PATCH)
- `headers`: (Optional) Key-value pairs for request headers
- `body`: (Optional) JSON payload for the request body
- `json_path`: (Optional) JSONPath selector for parsing the response

### Examples

1. Simple GET request:

```
~fetch http://localhost:3000/api/users
```

2. GET request with custom header:

```
~fetch http://localhost:3000/api/users GET Authorization="Bearer token123"
```

3. POST request with JSON body:

```
~fetch http://localhost:3000/api/users POST `{"name":"John Doe","email":"john@example.com"}`
```

4. GET request with JSONPath selector:

```
~fetch http://localhost:3000/api/users GET $.data[0].name
```

5. PUT request with headers and body:

```
~fetch http://localhost:3000/api/users/1 PUT "Content-Type"="application/json" `{"name":"Jane Doe"}`
```

7. PATCH request with body:

```
~fetch http://localhost:3000/api/users/1 PATCH `{"status":"inactive"}`
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is open source and available under the [MIT License](LICENSE).

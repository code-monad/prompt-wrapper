# Prompt Wrapper Service

A simple service that provides wise sayings from an LLM (OpenRouter) with rate limiting and caching.

## Features

- HTTP endpoints for retrieving sayings
- Rate limiting for users
- Caching of LLM responses
- Integration with OpenRouter for LLM capabilities

## Setup

1. Clone the repository
2. Copy `.env.example` to `.env` and update with your settings
3. Get an API key from [OpenRouter](https://openrouter.ai)
4. Update the `.env` file with your OpenRouter API key

## Running the service

```bash
cargo run
```

The service will be available at http://localhost:3000

## API Endpoints

### POST /saying

Creates a new saying using the OpenRouter LLM API and returns it.

**Request:**
```json
{
  "prompt": "Optional prompt to guide the LLM"
}
```

**Response:**
```json
{
  "id": "uuid",
  "content": "The saying content",
  "created_at": "2023-01-01T00:00:00Z",
  "source": "llm"
}
```

### GET /status

Returns the user's rate limit status and their last retrieved saying.

**Response:**
```json
{
  "can_query": true,
  "remaining_requests": 5,
  "reset_at": "2023-01-01T01:00:00Z",
  "last_saying": {
    "id": "uuid",
    "content": "The last saying content",
    "created_at": "2023-01-01T00:00:00Z",
    "source": "llm"
  }
}
```

## Configuration

All configuration is done through environment variables or the `.env` file:

- `SERVER_HOST`: Host to bind the server to
- `SERVER_PORT`: Port to bind the server to
- `OPENROUTER_API_KEY`: Your OpenRouter API key
- `OPENROUTER_MODEL`: The model to use (default: mistralai/mistral-7b-instruct)
- `RATE_LIMIT_MAX_REQUESTS`: Maximum number of requests per window
- `RATE_LIMIT_WINDOW_SECONDS`: Window size in seconds for rate limiting
- `STORAGE_TYPE`: Type of storage to use (memory, redis, sqlite)

## License

MIT 
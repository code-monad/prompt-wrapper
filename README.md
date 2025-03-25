# Prompt Wrapper Service

A simple service that provides wise sayings from an LLM (OpenRouter) with rate limiting and caching.

## Features

- RESTful HTTP endpoints for managing sayings and presets
- Rate limiting for users
- Persistent storage with Sled embedded database
- Rich UI-ready preset configurations
- Random preset selection for each user session
- Integration with OpenRouter for LLM capabilities
- Test user available in debug builds for easy testing

## Setup

1. Clone the repository
2. Copy `.env.example` to `.env` and update with your settings
3. Get an API key from [OpenRouter](https://openrouter.ai)
4. Update the `.env` file with your OpenRouter API key
5. Customize prompt presets in `presets.yaml` file

## Running the service

```bash
cargo run
```

The service will be available at http://localhost:3000

## API Endpoints

### Sayings Resource

#### GET /sayings

Returns a list of sayings for the specified user.

**Query Parameters:**
- `user_id` (optional): Identifier for the user. If not provided, a default value is used.
- `limit` (optional): Maximum number of sayings to return. Default is 10.

**Response:**
```json
[
  {
    "id": "uuid1",
    "content": "Saying content 1",
    "created_at": "2023-01-01T00:00:00Z",
    "source": "llm"
  },
  {
    "id": "uuid2",
    "content": "Saying content 2",
    "created_at": "2023-01-01T00:00:00Z",
    "source": "llm"
  }
]
```

#### GET /sayings/latest

Returns the latest saying for the specified user.

**Query Parameters:**
- `user_id` (optional): Identifier for the user. If not provided, a default value is used.

**Response:**
```json
{
  "id": "uuid",
  "content": "The saying content",
  "created_at": "2023-01-01T00:00:00Z",
  "source": "llm"
}
```

#### POST /sayings

Creates a new saying using the OpenRouter LLM API and returns it.

**Query Parameters:**
- `user_id` (optional): Identifier for the user. If not provided, a default value is used.

**Request Body:**
```json
{
  "prompt": "Optional prompt to guide the LLM",
  "preset_id": "Optional preset ID to use a specific preset"
}
```

If neither `prompt` nor `preset_id` is provided, the service will use the preset that was randomly selected for the user.

**Response:**
```json
{
  "id": "uuid",
  "content": "The saying content",
  "created_at": "2023-01-01T00:00:00Z",
  "source": "llm"
}
```

### User Status Resource

#### GET /users/{user_id}/status

Returns the user's rate limit status, their last retrieved saying, and their currently selected preset.

**Response:**
```json
{
  "user_id": "user123",
  "can_query": true,
  "remaining_requests": 5,
  "reset_at": "2023-01-01T01:00:00Z",
  "last_saying": {
    "id": "uuid",
    "content": "The last saying content",
    "created_at": "2023-01-01T00:00:00Z",
    "source": "llm"
  },
  "selected_preset": {
    "id": "oracle",
    "name": "Ape Oracle",
    "description": "Ancient wisdom for modern questions",
    "tags": ["oracle", "wisdom", "mystical"],
    "button_text": "Reveal My Answer",
    "loading_text": "Consulting the oracle...",
    "instruction_text": "Focus on your question, press the button, and receive wisdom from the Ape Oracle..."
  }
}
```

### Presets Resource

#### GET /presets

Returns all available presets.

**Response:**
```json
[
  {
    "id": "oracle",
    "name": "Ape Oracle",
    "description": "Ancient wisdom for modern questions",
    "tags": ["oracle", "wisdom", "mystical"],
    "button_text": "Reveal My Answer",
    "loading_text": "Consulting the oracle...",
    "instruction_text": "Focus on your question, press the button, and receive wisdom from the Ape Oracle..."
  },
  {
    "id": "fortune",
    "name": "Fortune Teller",
    "description": "Glimpse into your future",
    "tags": ["fortune", "future", "prediction"],
    "button_text": "Read My Fortune",
    "loading_text": "Gazing into the crystal ball...",
    "instruction_text": "Think of what you wish to know about your future, then press the button..."
  }
]
```

#### GET /presets/{preset_id}

Returns a specific preset by ID.

**Response:**
```json
{
  "id": "oracle",
  "name": "Ape Oracle",
  "description": "Ancient wisdom for modern questions",
  "tags": ["oracle", "wisdom", "mystical"],
  "button_text": "Reveal My Answer",
  "loading_text": "Consulting the oracle...",
  "instruction_text": "Focus on your question, press the button, and receive wisdom from the Ape Oracle..."
}
```

## Presets Configuration

Presets are defined in a YAML file specified by the `PRESETS_FILE_PATH` environment variable. Each preset contains:

- `id`: Unique identifier for the preset
- `name`: Display name for the preset
- `description`: Short description of the preset's purpose
- `tags`: List of category tags
- `button_text`: Text to display on action buttons
- `loading_text`: Text to display during loading/processing
- `instruction_text`: Guidance text for users
- `system_prompt`: The system prompt to set the context for the LLM
- `user_prompts`: List of possible user prompts that will be randomly selected

Example preset configuration:

```yaml
- id: oracle
  name: Ape Oracle
  description: Ancient wisdom for modern questions
  tags:
    - oracle
    - wisdom
    - mystical
  button_text: Reveal My Answer
  loading_text: Consulting the oracle...
  instruction_text: Focus on your question, press the button, and receive wisdom from the Ape Oracle...
  system_prompt: >
    You are the Ape Oracle, a mystical entity that provides profound, 
    wise, and sometimes cryptic answers to unspoken questions.
  user_prompts:
    - "Will I find success?"
    - "What should I do next?"
    - "Is this the right path?"
```

## Configuration

All configuration is done through environment variables or the `.env` file:

- `SERVER_HOST`: Host to bind the server to
- `SERVER_PORT`: Port to bind the server to
- `OPENROUTER_API_KEY`: Your OpenRouter API key
- `OPENROUTER_MODEL`: The model to use (default: mistralai/mistral-7b-instruct)
- `RATE_LIMIT_MAX_REQUESTS`: Maximum number of requests per window
- `RATE_LIMIT_WINDOW_SECONDS`: Window size in seconds for rate limiting
- `STORAGE_TYPE`: Type of storage to use (memory, sled, redis, sqlite)
- `STORAGE_CONNECTION_STRING`: Connection string for the storage
- `PRESETS_FILE_PATH`: Path to the presets YAML file

## Development Features

### Test User

In debug builds (when compiling without the `--release` flag), a test user is automatically initialized with:

- User ID: `test_user` 
- Empty initial state (no pre-populated sayings)
- Fresh rate limit quota (according to configured settings)

The test user follows a completely dynamic workflow identical to regular users:
- All sayings are generated on-demand via the LLM or cache
- Rate limiting follows standard rules
- Presets are dynamically selected for each user session
- No hardcoded or static responses are used

You can use this test user for development and testing purposes by including `user_id=test_user` in your requests:

```
GET /sayings?user_id=test_user
```

**Note:** This test user ID is blocked in release/production builds to prevent misuse in production environments.
To test in release mode, use a different user ID.

## License

MIT 
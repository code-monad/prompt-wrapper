# API Documentation

## English

### Endpoints

#### Get Sayings
```http
GET /sayings?user_id={user_id}&limit={limit}&language_id={language_id}
```
Retrieves a list of sayings for a user.

**Query Parameters:**
- `user_id` (optional): The user identifier. Defaults to "default_user"
- `limit` (optional): Maximum number of sayings to return. Defaults to 10
- `language_id` (optional): Language code for translation. Defaults to "en"

**Response:**
```json
[
  {
    "id": "string",
    "content": "string",
    "created_at": "ISO datetime",
    "source": "llm|cache|database"
  }
]
```

#### Get Latest Saying
```http
GET /sayings/latest?user_id={user_id}&language_id={language_id}
```
Retrieves the latest saying for a user.

**Query Parameters:**
- `user_id` (optional): The user identifier. Defaults to "default_user"
- `language_id` (optional): Language code for translation. Defaults to "en"

**Response:**
```json
{
  "id": "string",
  "content": "string",
  "created_at": "ISO datetime",
  "source": "llm|cache|database"
}
```

#### Create Saying
```http
POST /sayings?user_id={user_id}&language_id={language_id}
```
Creates a new saying for a user.

**Query Parameters:**
- `user_id` (optional): The user identifier. Defaults to "default_user"
- `language_id` (optional): Language code for translation. Defaults to "en"

**Request Body:**
```json
{
  "prompt": "string (optional)",
  "preset_id": "string (optional)",
  "language_id": "string (optional)"
}
```
Either provide a prompt or a preset_id. If neither is provided, a preset will be selected for the user automatically.
The language_id can be specified in either the query or the request body.

**Response:**
```json
{
  "id": "string",
  "content": "string",
  "created_at": "ISO datetime",
  "source": "llm|cache|database"
}
```

**Note:** If a user is rate-limited, the system will return a cached saying instead of generating a new one.

#### Get User Status
```http
GET /users/{user_id}/status
```
Retrieves status information for a user.

**Response:**
```json
{
  "user_id": "string",
  "can_query": boolean,
  "remaining_requests": number,
  "reset_at": "ISO datetime (optional)",
  "last_saying": {
    "id": "string",
    "content": "string",
    "created_at": "ISO datetime",
    "source": "llm|cache|database"
  } (optional),
  "selected_preset": {
    "id": "string",
    "name": "string",
    "description": "string",
    "tags": ["string"],
    "button_text": "string",
    "loading_text": "string",
    "instruction_text": "string"
  } (optional)
}
```

#### Get All Presets
```http
GET /presets
```
Retrieves all available presets.

**Response:**
```json
[
  {
    "id": "string",
    "name": "string",
    "description": "string",
    "tags": ["string"],
    "button_text": "string",
    "loading_text": "string",
    "instruction_text": "string"
  }
]
```

#### Get Preset by ID
```http
GET /presets/{preset_id}
```
Retrieves a specific preset by ID.

**Response:**
```json
{
  "id": "string",
  "name": "string",
  "description": "string",
  "tags": ["string"],
  "button_text": "string",
  "loading_text": "string",
  "instruction_text": "string"
}
```

#### Get All Languages
```http
GET /languages
```
Retrieves all supported languages.

**Response:**
```json
[
  {
    "id": "string",
    "name": "string",
    "native_name": "string"
  }
]
```

#### Get Language by ID
```http
GET /languages/{language_id}
```
Retrieves a specific language by ID.

**Response:**
```json
{
  "id": "string",
  "name": "string",
  "native_name": "string"
}
```

### Global Cache System

Sayings with the same prompt and preset combination are cached across all users. This means:
- Similar requests from different users may receive the same response from cache
- The system prioritizes cache responses for rate-limited users
- Random determination is used to decide whether to use cache or LLM for new requests

### Multi-language Support

The system supports multiple languages through LLM translation:
- Specify a language_id in the request to get responses in that language
- For non-English languages, the response will include both English and translated versions
- The system does not store separate translations in the cache; they are generated on-the-fly

## 中文

### 接口列表

#### 获取一日一句列表
```http
GET /sayings?user_id={user_id}&limit={limit}&language_id={language_id}
```
获取用户的一日一句列表。

**查询参数：**
- `user_id` (可选)：用户标识符。默认为 "default_user"
- `limit` (可选)：返回的最大一日一句数量。默认为10
- `language_id` (可选)：翻译的语言代码。默认为 "en"（英语）

**响应：**
```json
[
  {
    "id": "字符串",
    "content": "字符串",
    "created_at": "ISO 日期时间",
    "source": "llm|cache|database"
  }
]
```

#### 获取最新一日一句
```http
GET /sayings/latest?user_id={user_id}&language_id={language_id}
```
获取用户的最新一日一句。

**查询参数：**
- `user_id` (可选)：用户标识符。默认为 "default_user"
- `language_id` (可选)：翻译的语言代码。默认为 "en"（英语）

**响应：**
```json
{
  "id": "字符串",
  "content": "字符串",
  "created_at": "ISO 日期时间",
  "source": "llm|cache|database"
}
```

#### 创建一日一句
```http
POST /sayings?user_id={user_id}&language_id={language_id}
```
为用户创建一个新一日一句。

**查询参数：**
- `user_id` (可选)：用户标识符。默认为 "default_user"
- `language_id` (可选)：翻译的语言代码。默认为 "en"（英语）

**请求体：**
```json
{
  "prompt": "字符串 (可选)",
  "preset_id": "字符串 (可选)",
  "language_id": "字符串 (可选)"
}
```
提供一个提示词或预设ID。如果两者都未提供，系统将自动为用户选择一个预设。
语言ID可以在查询参数或请求体中指定。

**响应：**
```json
{
  "id": "字符串",
  "content": "字符串",
  "created_at": "ISO 日期时间",
  "source": "llm|cache|database"
}
```

**注意：** 如果用户被限流，系统将返回缓存的一日一句而不是生成新的。

#### 获取用户状态
```http
GET /users/{user_id}/status
```
获取用户的状态信息。

**响应：**
```json
{
  "user_id": "字符串",
  "can_query": 布尔值,
  "remaining_requests": 数字,
  "reset_at": "ISO 日期时间 (可选)",
  "last_saying": {
    "id": "字符串",
    "content": "字符串",
    "created_at": "ISO 日期时间",
    "source": "llm|cache|database"
  } (可选),
  "selected_preset": {
    "id": "字符串",
    "name": "字符串",
    "description": "字符串",
    "tags": ["字符串"],
    "button_text": "字符串",
    "loading_text": "字符串",
    "instruction_text": "字符串"
  } (可选)
}
```

#### 获取所有预设
```http
GET /presets
```
获取所有可用的预设。

**响应：**
```json
[
  {
    "id": "字符串",
    "name": "字符串",
    "description": "字符串",
    "tags": ["字符串"],
    "button_text": "字符串",
    "loading_text": "字符串",
    "instruction_text": "字符串"
  }
]
```

#### 通过ID获取预设
```http
GET /presets/{preset_id}
```
通过ID获取特定预设。

**响应：**
```json
{
  "id": "字符串",
  "name": "字符串",
  "description": "字符串",
  "tags": ["字符串"],
  "button_text": "字符串",
  "loading_text": "字符串",
  "instruction_text": "字符串"
}
```

#### 获取所有支持的语言
```http
GET /languages
```
获取所有支持的语言。

**响应：**
```json
[
  {
    "id": "字符串",
    "name": "字符串",
    "native_name": "字符串"
  }
]
```

#### 通过ID获取语言
```http
GET /languages/{language_id}
```
通过ID获取特定语言。

**响应：**
```json
{
  "id": "字符串",
  "name": "字符串",
  "native_name": "字符串"
}
```

### 全局缓存系统

具有相同提示词和预设组合的一日一句会在所有用户之间共享缓存。这意味着：
- 来自不同用户的相似请求可能会从缓存中收到相同的响应
- 系统优先为被限流的用户提供缓存响应
- 系统使用随机决定是否为新请求使用缓存或LLM

### 多语言支持

系统通过LLM翻译支持多种语言：
- 在请求中指定language_id以获取该语言的响应
- 对于非英语语言，响应将同时包含英语和翻译版本
- 系统不会在缓存中存储单独的翻译版本；它们是即时生成的 
# X-Request-Id 找回机制 - Apifox 测试用例

> 用于在 Apifox 中手动测试 X-Request-Id 注入 + `/log/get-request` 找回接口。

## 环境变量

在 Apifox 中设置以下环境变量：

| 变量名 | 值 | 说明 |
|--------|-----|------|
| `baseUrl` | `https://api.tu-zi.com` | API 站地址 |
| `apiKey` | `sk-xxxx` | 你的 API Key |
| `requestId` | （自动生成或手动填） | UUID 格式 |

---

## 用例 1：正常生图 + X-Request-Id 注入

**目的**：验证生图接口能正确接受 `X-Request-Id` 请求头。

### 请求

```
POST {{baseUrl}}/v1/images/generations
```

**Headers:**

| Key | Value |
|-----|-------|
| Authorization | Bearer {{apiKey}} |
| Content-Type | application/json |
| X-Request-Id | {{requestId}} |

> `requestId` 可以在 Apifox 的"前置脚本"中自动生成：
> ```js
> pm.environment.set("requestId", pm.variables.replaceIn("{{$guid}}"));
> ```

**Body (JSON):**

```json
{
  "prompt": "一只橘猫坐在窗台上，窗外是下雨天",
  "model": "gpt-image-1",
  "size": "1024x1024"
}
```

### 期望响应

**Status:** 200

```json
{
  "data": [
    {
      "url": "https://xxx.com/generated-image.png"
    }
  ]
}
```

### 后置脚本（断言）

```js
pm.test("状态码 200", function () {
  pm.response.to.have.status(200);
});

pm.test("返回 data 数组且有 url", function () {
  const json = pm.response.json();
  pm.expect(json.data).to.be.an("array");
  pm.expect(json.data.length).to.be.greaterThan(0);
  pm.expect(json.data[0].url).to.be.a("string");
});

// 保存 requestId 以供用例 2 使用
console.log("X-Request-Id:", pm.environment.get("requestId"));
```

---

## 用例 2：通过 X-Request-Id 找回图片

**目的**：验证 `/log/get-request` 能通过 requestId 找回已生成的图片。

### 前置条件

先执行用例 1（正常生图），记录下该次的 `requestId`。

### 请求

```
GET {{baseUrl}}/log/get-request?id={{requestId}}
```

**Headers:**

| Key | Value |
|-----|-------|
| Authorization | Bearer {{apiKey}} |

### 期望响应（成功找回）

**Status:** 200

```json
{
  "status": "succeeded",
  "data": [
    {
      "url": "https://xxx.com/generated-image.png"
    }
  ]
}
```

### 后置脚本（断言）

```js
pm.test("状态码 200", function () {
  pm.response.to.have.status(200);
});

pm.test("status 为 succeeded 或 success", function () {
  const json = pm.response.json();
  pm.expect(["succeeded", "success"]).to.include(json.status);
});

pm.test("data 数组包含 url", function () {
  const json = pm.response.json();
  pm.expect(json.data).to.be.an("array");
  pm.expect(json.data.length).to.be.greaterThan(0);
  pm.expect(json.data[0].url).to.be.a("string");
  pm.expect(json.data[0].url).to.match(/^https?:\/\//);
});
```

---

## 用例 3：无效 requestId 找回（应失败）

**目的**：验证不存在的 requestId 找回接口返回合理的错误。

### 请求

```
GET {{baseUrl}}/log/get-request?id=non-existent-id-12345
```

**Headers:**

| Key | Value |
|-----|-------|
| Authorization | Bearer {{apiKey}} |

### 期望响应

**Status:** 200 或 404（取决于 API 站实现）

可能的返回：

```json
{
  "status": "not_found"
}
```

或：

```json
{
  "status": "failed",
  "error": "请求不存在"
}
```

### 后置脚本（断言）

```js
pm.test("状态码为 200 或 404", function () {
  pm.expect([200, 404]).to.include(pm.response.code);
});

pm.test("status 非 succeeded/success（找回应失败）", function () {
  if (pm.response.code === 200) {
    const json = pm.response.json();
    if (json.status) {
      pm.expect(["succeeded", "success"]).to.not.include(json.status);
    }
  }
});
```

---

## 用例 4：缺少 id 参数

**目的**：验证找回接口对缺少参数的容错。

### 请求

```
GET {{baseUrl}}/log/get-request
```

**Headers:**

| Key | Value |
|-----|-------|
| Authorization | Bearer {{apiKey}} |

### 期望响应

**Status:** 400 或 422

### 后置脚本

```js
pm.test("缺少 id 应返回 4xx", function () {
  pm.expect(pm.response.code).to.be.within(400, 499);
});
```

---

## 用例 5：端到端超时模拟（完整链路）

**目的**：模拟客户端实际的超时找回流程。

### 步骤

这是一个**流程测试**，在 Apifox 中设为"测试套件"按顺序执行：

#### Step 1 - 生图（带 X-Request-Id）

同用例 1，前置脚本生成 UUID：

```js
const uuid = pm.variables.replaceIn("{{$guid}}");
pm.environment.set("requestId", uuid);
console.log("生成的 X-Request-Id:", uuid);
```

#### Step 2 - 等待 3 秒

在 Apifox 测试套件中加一个延时步骤（3000ms），模拟"生图已完成但客户端超时"的间隔。

#### Step 3 - 用相同 requestId 找回

同用例 2，直接读 `{{requestId}}`。

#### 断言

```js
pm.test("找回的 URL 与原始生图的 URL 一致", function () {
  const json = pm.response.json();
  const recoveredUrl = json.data?.[0]?.url;
  // 如果用例1把 url 存下来了可以对比
  const originalUrl = pm.environment.get("originalImageUrl");
  if (originalUrl) {
    pm.expect(recoveredUrl).to.equal(originalUrl);
  } else {
    pm.expect(recoveredUrl).to.be.a("string");
  }
});
```

> 用例 1 的后置脚本加一行保存 URL：
> ```js
> const url = pm.response.json()?.data?.[0]?.url;
> if (url) pm.environment.set("originalImageUrl", url);
> ```

---

## 用例 6：X-Request-Id 不影响无该头的正常请求

**目的**：验证不带 X-Request-Id 时，生图接口行为完全不变。

### 请求

```
POST {{baseUrl}}/v1/images/generations
```

**Headers:**

| Key | Value |
|-----|-------|
| Authorization | Bearer {{apiKey}} |
| Content-Type | application/json |

> 注意：故意**不带** X-Request-Id

**Body:**

```json
{
  "prompt": "一片星空下的海边",
  "model": "gpt-image-1",
  "size": "1024x1024"
}
```

### 期望响应

**Status:** 200，正常返回图片（与带 X-Request-Id 无区别）

### 后置脚本

```js
pm.test("不带 X-Request-Id 也能正常生图", function () {
  pm.response.to.have.status(200);
  const json = pm.response.json();
  pm.expect(json.data[0].url).to.be.a("string");
});
```

---

## 用例 7：验证 CORS 预检（浏览器场景专用）

**目的**：确认 API 站的 CORS 配置是否允许 `X-Request-Id`。

> 此测试在 Apifox 中模拟 OPTIONS 预检请求。

### 请求

```
OPTIONS {{baseUrl}}/v1/images/generations
```

**Headers:**

| Key | Value |
|-----|-------|
| Origin | http://localhost:7200 |
| Access-Control-Request-Method | POST |
| Access-Control-Request-Headers | authorization, content-type, x-request-id |

### 期望响应

**Status:** 200 或 204

### 后置脚本

```js
pm.test("CORS 允许 X-Request-Id", function () {
  const allowedHeaders = pm.response.headers.get("Access-Control-Allow-Headers");
  pm.expect(allowedHeaders?.toLowerCase()).to.include("x-request-id");
});

// 如果这个断言失败，说明 API 站 CORS 还没加 X-Request-Id
// 本地开发需要走 vite proxy 绕过
pm.test("打印 CORS 响应头（调试用）", function () {
  console.log("Allow-Headers:", pm.response.headers.get("Access-Control-Allow-Headers"));
  console.log("Allow-Origin:", pm.response.headers.get("Access-Control-Allow-Origin"));
  console.log("Allow-Methods:", pm.response.headers.get("Access-Control-Allow-Methods"));
});
```

---

## 测试套件运行顺序

在 Apifox 中创建**自动化测试**，按以下顺序编排：

| 序号 | 用例 | 说明 |
|------|------|------|
| 1 | 用例 7 | CORS 检查（可选，了解现状） |
| 2 | 用例 6 | 无 X-Request-Id 基线验证 |
| 3 | 用例 1 | 正常生图 + X-Request-Id |
| 4 | 用例 2 | 找回（用上一步的 requestId） |
| 5 | 用例 3 | 无效 ID 找回 |
| 6 | 用例 4 | 缺少参数 |
| 7 | 用例 5 | 端到端流程 |

---

## 快速手动测试（cURL）

如果不想开 Apifox，直接在终端跑：

```bash
# 1. 生图（带 X-Request-Id）
REQUEST_ID=$(uuidgen)
echo "X-Request-Id: $REQUEST_ID"

curl -X POST "https://api.tu-zi.com/v1/images/generations" \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -H "X-Request-Id: $REQUEST_ID" \
  -d '{"prompt":"一只橘猫","model":"gpt-image-1","size":"1024x1024"}'

# 2. 找回
curl "https://api.tu-zi.com/log/get-request?id=$REQUEST_ID" \
  -H "Authorization: Bearer YOUR_API_KEY"
```

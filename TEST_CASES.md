# Vibe MCP Server - 测试用例

## API 端点概览

| 端点 | 方法 | 认证 | 功能 |
|------|------|------|------|
| `/health` | GET | 无 | 健康检查 |
| `/api/auth/register` | POST | 无 | 用户注册 |
| `/api/auth/login` | POST | 无 | 用户登录 |
| `/api/auth/verify` | POST | JWT | 邮箱验证 |
| `/api/auth/send-verification` | POST | JWT | 发送验证码 |
| `/api/auth/refresh-api-key` | POST | JWT | 刷新API Key |
| `/api/status` | GET | JWT/API-Key | 获取当前任务 |
| `/api/history` | GET | JWT/API-Key | 获取历史任务 |
| `/api/task/start` | POST | JWT | 开始任务 |
| `/api/task/update_state` | POST | JWT | 更新任务状态 |
| `/api/task/update_progress` | POST | JWT | 更新任务进度 |
| `/api/task/sync` | POST | JWT | 同步任务 |
| `/api/task/delete` | POST | JWT | 删除任务 |
| `/api/task/reset` | POST | JWT | 重置任务 |
| `/api/task/{id}/stages` | GET | JWT | 获取任务阶段历史 |
| `/api/task/mcp` | POST | JWT | MCP JSON-RPC |

---

## 测试用例

### TC-001: 健康检查

**目的**: 验证服务可用

```bash
# 正常情况
curl -s http://43.143.135.49:3010/health
# 期望: "OK"

# 负载下仍可访问
for i in {1..100}; do curl -s http://43.143.135.49:3010/health & done
wait
# 期望: 全部返回 "OK"
```

---

### TC-002: 用户注册

**目的**: 验证用户注册功能

```bash
# TC-002.1: 正常注册
curl -s -X POST http://43.143.135.49:3010/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email":"newuser@test.com","password":"test123456","password_confirm":"test123456"}'
# 期望: 201 Created, 返回 JWT token

# TC-002.2: 邮箱已存在
curl -s -X POST http://43.143.135.49:3010/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email":"autoglm@test.com","password":"test123456","password_confirm":"test123456"}'
# 期望: 400 Bad Request

# TC-002.3: 无效邮箱格式
curl -s -X POST http://43.143.135.49:3010/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email":"invalid-email","password":"test123456","password_confirm":"test123456"}'
# 期望: 400 Bad Request

# TC-002.4: 密码太短 (<6字符)
curl -s -X POST http://43.143.135.49:3010/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email":"test2@test.com","password":"123","password_confirm":"123"}'
# 期望: 400 Bad Request

# TC-002.5: 密码不匹配
curl -s -X POST http://43.143.135.49:3010/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email":"test3@test.com","password":"test123456","password_confirm":"test123457"}'
# 期望: 400 Bad Request

# TC-002.6: 空邮箱
curl -s -X POST http://43.143.135.49:3010/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email":"","password":"test123456","password_confirm":"test123456"}'
# 期望: 400 Bad Request
```

---

### TC-003: 用户登录

**目的**: 验证登录功能

```bash
# TC-003.1: 正常登录
curl -s -X POST http://43.143.135.49:3010/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"autoglm@test.com","password":"test123456"}'
# 期望: 200 OK, 返回 JWT token + user_id + api_key

# TC-003.2: 错误密码
curl -s -X POST http://43.143.135.49:3010/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"autoglm@test.com","password":"wrongpassword"}'
# 期望: 401 Unauthorized

# TC-003.3: 不存在的邮箱
curl -s -X POST http://43.143.135.49:3010/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"nonexistent@test.com","password":"test123456"}'
# 期望: 401 Unauthorized

# TC-003.4: 空密码
curl -s -X POST http://43.143.135.49:3010/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"autoglm@test.com","password":""}'
# 期望: 400 Bad Request
```

---

### TC-004: 任务状态获取 (GET /api/status)

**目的**: 验证获取当前任务列表

```bash
# TC-004.1: JWT认证获取任务
TOKEN="eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..."
curl -s http://43.143.135.49:3010/api/status \
  -H "Authorization: Bearer $TOKEN"
# 期望: 200 OK, {"taskCount":N,"tasks":[...]}

# TC-004.2: API Key认证获取任务
curl -s http://43.143.135.49:3010/api/status \
  -H "X-API-Key: vibe-mcp-secret-key"
# 期望: 200 OK

# TC-004.3: 无认证
curl -s http://43.143.135.49:3010/api/status
# 期望: 401 Unauthorized

# TC-004.4: 过期Token
curl -s http://43.143.135.49:3010/api/status \
  -H "Authorization: Bearer expired_token_here"
# 期望: 401 Unauthorized

# TC-004.5: 无效Token
curl -s http://43.143.135.49:3010/api/status \
  -H "Authorization: Bearer invalid_token"
# 期望: 401 Unauthorized
```

---

### TC-005: 更新任务状态 (POST /api/task/update_state)

**目的**: 验证任务状态更新

```bash
# TC-005.1: 更新为running
curl -s -X POST http://43.143.135.49:3010/api/task/update_state \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"task_id":"test-001","status":"running"}'
# 期望: {"status":"ok"}

# TC-005.2: 更新为armed
curl -s -X POST http://43.143.135.49:3010/api/task/update_state \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"task_id":"test-001","status":"armed"}'
# 期望: {"status":"ok"}

# TC-005.3: 更新为completed
curl -s -X POST http://43.143.135.49:3010/api/task/update_state \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"task_id":"test-001","status":"completed"}'
# 期望: {"status":"ok"}

# TC-005.4: 更新为failed
curl -s -X POST http://43.143.135.49:3010/api/task/update_state \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"task_id":"test-001","status":"failed"}'
# 期望: {"status":"ok"}

# TC-005.5: 无效状态
curl -s -X POST http://43.143.135.49:3010/api/task/update_state \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"task_id":"test-001","status":"invalid_status"}'
# 期望: 400 Bad Request

# TC-005.6: 缺少task_id
curl -s -X POST http://43.143.135.49:3010/api/task/update_state \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"status":"running"}'
# 期望: 400 Bad Request

# TC-005.7: 不存在的task_id
curl -s -X POST http://43.143.135.49:3010/api/task/update_state \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"task_id":"nonexistent-task","status":"running"}'
# 期望: 404 Not Found 或处理成功(幂等)

# TC-005.8: 带current_stage更新
curl -s -X POST http://43.143.135.49:3010/api/task/update_state \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"task_id":"test-001","status":"running","current_stage":"coding"}'
# 期望: {"status":"ok"}
```

---

### TC-006: 更新任务进度 (POST /api/task/update_progress)

**目的**: 验证进度更新

```bash
# TC-006.1: 正常更新进度
curl -s -X POST http://43.143.135.49:3010/api/task/update_progress \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"task_id":"test-001","progress":50}'
# 期望: {"status":"ok"}

# TC-006.2: 进度0
curl -s -X POST http://43.143.135.49:3010/api/task/update_progress \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"task_id":"test-001","progress":0}'
# 期望: {"status":"ok"}

# TC-006.3: 进度100
curl -s -X POST http://43.143.135.49:3010/api/task/update_progress \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"task_id":"test-001","progress":100}'
# 期望: {"status":"ok"}

# TC-006.4: 进度超过100 (应拦截)
curl -s -X POST http://43.143.135.49:3010/api/task/update_progress \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"task_id":"test-001","progress":150}'
# 期望: 400 Bad Request

# TC-006.5: 负数进度 (应拦截)
curl -s -X POST http://43.143.135.49:3010/api/task/update_progress \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"task_id":"test-001","progress":-10}'
# 期望: 400 Bad Request

# TC-006.6: 非数字进度 (应拦截)
curl -s -X POST http://43.143.135.49:3010/api/task/update_progress \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"task_id":"test-001","progress":"fifty"}'
# 期望: 400 Bad Request

# TC-006.7: 缺少progress字段
curl -s -X POST http://43.143.135.49:3010/api/task/update_progress \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"task_id":"test-001"}'
# 期望: 400 Bad Request

# TC-006.8: 带current_stage更新
curl -s -X POST http://43.143.135.49:3010/api/task/update_progress \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"task_id":"test-001","progress":75,"current_stage":"testing"}'
# 期望: {"status":"ok"}
```

---

### TC-007: 同步任务 (POST /api/task/sync)

**目的**: 验证任务同步

```bash
# TC-007.1: 正常同步新任务
curl -s -X POST http://43.143.135.49:3010/api/task/sync \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "id": "sync-test-001",
    "name": "Sync Test Task",
    "status": "running",
    "ide": "VSCode",
    "source": "desktop",
    "window_title": "test.py - VSCode",
    "active_file": "/path/to/test.py",
    "project_path": "/path/to/project"
  }'
# 期望: {"status":"ok","task":{...}}

# TC-007.2: 同步已存在任务 (更新)
curl -s -X POST http://43.143.135.49:3010/api/task/sync \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "id": "sync-test-001",
    "name": "Sync Test Task Updated",
    "status": "running",
    "ide": "VSCode"
  }'
# 期望: {"status":"ok","task":{...}}

# TC-007.3: 同步带description
curl -s -X POST http://43.143.135.49:3010/api/task/sync \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "id": "sync-test-002",
    "name": "Task With Desc",
    "status": "running",
    "description": "This is a test task description"
  }'
# 期望: {"status":"ok"}

# TC-007.4: 缺少必填字段 (id)
curl -s -X POST http://43.143.135.49:3010/api/task/sync \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"Test","status":"running"}'
# 期望: 400 Bad Request

# TC-007.5: 无效status值
curl -s -X POST http://43.143.135.49:3010/api/task/sync \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"id":"test-xxx","name":"Test","status":"invalid"}'
# 期望: 400 Bad Request
```

---

### TC-008: 删除任务 (POST /api/task/delete)

**目的**: 验证任务删除

```bash
# TC-008.1: 正常删除
curl -s -X POST http://43.143.135.49:3010/api/task/delete \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"task_id":"test-to-delete"}'
# 期望: {"status":"ok"} 或 200 OK

# TC-008.2: 删除不存在的任务
curl -s -X POST http://43.143.135.49:3010/api/task/delete \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"task_id":"nonexistent-task"}'
# 期望: 404 Not Found

# TC-008.3: 缺少task_id
curl -s -X POST http://43.143.135.49:3010/api/task/delete \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{}'
# 期望: 400 Bad Request

# TC-008.4: 无认证删除
curl -s -X POST http://43.143.135.49:3010/api/task/delete \
  -H "Content-Type: application/json" \
  -d '{"task_id":"test-001"}'
# 期望: 401 Unauthorized

# TC-008.5: 删除后验证任务消失
TASK_ID="test-delete-verify"
# 先创建任务
curl -s -X POST http://43.143.135.49:3010/api/task/sync \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"id\":\"$TASK_ID\",\"name\":\"To Be Deleted\",\"status\":\"running\"}"
# 验证存在
curl -s http://43.143.135.49:3010/api/status -H "Authorization: Bearer $TOKEN" | grep "$TASK_ID"
# 删除
curl -s -X POST http://43.143.135.49:3010/api/task/delete \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"task_id\":\"$TASK_ID\"}"
# 验证不存在
curl -s http://43.143.135.49:3010/api/status -H "Authorization: Bearer $TOKEN" | grep "$TASK_ID"
# 期望: 第二次grep无输出
```

---

### TC-009: 历史记录 (GET /api/history)

**目的**: 验证历史任务查询

```bash
# TC-009.1: 获取历史记录
curl -s http://43.143.135.49:3010/api/history \
  -H "Authorization: Bearer $TOKEN"
# 期望: 200 OK, {"taskCount":N,"tasks":[...]}

# TC-009.2: API Key访问历史
curl -s http://43.143.135.49:3010/api/history \
  -H "X-API-Key: vibe-mcp-secret-key"
# 期望: 200 OK

# TC-009.3: 无认证
curl -s http://43.143.135.49:3010/api/history
# 期望: 401 Unauthorized

# TC-009.4: 验证已完成任务在历史中
# 先将任务设为completed
curl -s -X POST http://43.143.135.49:3010/api/task/update_state \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"task_id":"test-001","status":"completed"}'
# 查询历史
curl -s http://43.143.135.49:3010/api/history \
  -H "Authorization: Bearer $TOKEN" | grep "test-001"
# 期望: 找到该任务
```

---

### TC-010: 阶段历史 (GET /api/task/{id}/stages)

**目的**: 验证任务阶段历史

```bash
# TC-010.1: 获取任务阶段历史
curl -s http://43.143.135.49:3010/api/task/test-001/stages \
  -H "Authorization: Bearer $TOKEN"
# 期望: 200 OK, {"stages":[...]}

# TC-010.2: 不存在的任务
curl -s http://43.143.135.49:3010/api/task/nonexistent/stages \
  -H "Authorization: Bearer $TOKEN"
# 期望: 404 Not Found

# TC-010.3: 无认证
curl -s http://43.143.135.49:3010/api/task/test-001/stages
# 期望: 401 Unauthorized

# TC-010.4: 更新进度后验证阶段历史增加
curl -s -X POST http://43.143.135.49:3010/api/task/update_progress \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"task_id":"test-001","progress":25,"current_stage":"planning"}'
curl -s http://43.143.135.49:3010/api/task/test-001/stages \
  -H "Authorization: Bearer $TOKEN" | jq '.stages | length'
# 期望: 数量增加
```

---

### TC-011: 重置任务 (POST /api/task/reset)

**目的**: 验证任务重置

```bash
# TC-011.1: 正常重置
curl -s -X POST http://43.143.135.49:3010/api/task/reset \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"task_id":"test-001"}'
# 期望: {"status":"ok"}

# TC-011.2: 缺少task_id
curl -s -X POST http://43.143.135.49:3010/api/task/reset \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{}'
# 期望: 400 Bad Request

# TC-011.3: 重置后验证状态
# 重置任务
curl -s -X POST http://43.143.135.49:3010/api/task/reset \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"task_id":"test-001"}'
# 查询状态
curl -s http://43.143.135.49:3010/api/status \
  -H "Authorization: Bearer $TOKEN" | jq '.tasks[] | select(.id=="test-001") | .status'
# 期望: "running" 或 "armed"
```

---

### TC-012: JWT Token 验证

**目的**: 验证Token处理

```bash
# TC-012.1: 过期Token
EXPIRED_TOKEN="eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiI5YzAwY2RiYSIsImVtYWlsIjoiYXV0b2dsbUB0ZXN0LmNvbSIsImV4cCI6MTc3NDg3MTYwMH0.xxx"
curl -s http://43.143.135.49:3010/api/status \
  -H "Authorization: Bearer $EXPIRED_TOKEN"
# 期望: 401 Unauthorized

# TC-012.2: 损坏的Token
curl -s http://43.143.135.49:3010/api/status \
  -H "Authorization: Bearer invalid.token.here"
# 期望: 401 Unauthorized

# TC-012.3: 空Token
curl -s http://43.143.135.49:3010/api/status \
  -H "Authorization: Bearer "
# 期望: 401 Unauthorized

# TC-012.4: 缺少Bearer前缀
curl -s http://43.143.135.49:3010/api/status \
  -H "Authorization: $TOKEN"
# 期望: 401 Unauthorized
```

---

### TC-013: API Key 验证

**目的**: 验证API Key认证

```bash
# TC-013.1: 正确API Key
curl -s http://43.143.135.49:3010/api/status \
  -H "X-API-Key: vibe-mcp-secret-key"
# 期望: 200 OK

# TC-013.2: 错误API Key
curl -s http://43.143.135.49:3010/api/status \
  -H "X-API-Key: wrong-api-key"
# 期望: 401 Unauthorized

# TC-013.3: 空API Key
curl -s http://43.143.135.49:3010/api/status \
  -H "X-API-Key: "
# 期望: 401 Unauthorized

# TC-013.4: API Key用于写操作 (应失败)
curl -s -X POST http://43.143.135.49:3010/api/task/update_state \
  -H "X-API-Key: vibe-mcp-secret-key" \
  -H "Content-Type: application/json" \
  -d '{"task_id":"test-001","status":"running"}'
# 期望: 401 Unauthorized (API Key只能读)
```

---

### TC-014: 并发测试

**目的**: 验证并发处理能力

```bash
# TC-014.1: 100并发请求状态
for i in {1..100}; do
  curl -s http://43.143.135.49:3010/api/status \
    -H "Authorization: Bearer $TOKEN" &
done
wait
# 期望: 全部返回200, 无超时

# TC-014.2: 10并发更新进度
for i in {1..10}; do
  curl -s -X POST http://43.143.135.49:3010/api/task/update_progress \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d "{\"task_id\":\"test-001\",\"progress\":$((i*10))}" &
done
wait
# 期望: 全部成功, 无数据竞争

# TC-014.3: 快速连续更新同一任务
for i in {1..20}; do
  curl -s -X POST http://43.143.135.49:3010/api/task/update_state \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d "{\"task_id\":\"test-001\",\"status\":\"running\",\"current_stage\":\"stage-$i\"}" &
done
wait
# 期望: 全部成功, 最终状态一致
```

---

### TC-015: 性能测试

**目的**: 验证响应时间

```bash
# TC-015.1: /health 响应时间
time curl -s http://43.143.135.49:3010/health
# 期望: < 100ms

# TC-015.2: /api/status 响应时间 (有任务)
time curl -s http://43.143.135.49:3010/api/status \
  -H "Authorization: Bearer $TOKEN"
# 期望: < 500ms

# TC-015.3: /api/history 响应时间
time curl -s http://43.143.135.49:3010/api/history \
  -H "Authorization: Bearer $TOKEN"
# 期望: < 500ms

# TC-015.4: /api/task/update_progress 响应时间
time curl -s -X POST http://43.143.135.49:3010/api/task/update_progress \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"task_id":"test-001","progress":50}'
# 期望: < 500ms

# TC-015.5: /api/task/sync 响应时间
time curl -s -X POST http://43.143.135.49:3010/api/task/sync \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"id":"perf-test","name":"Perf Test","status":"running"}'
# 期望: < 500ms
```

---

## 自动化测试脚本

```bash
#!/bin/bash
# test_api.sh - API 自动化测试

BASE_URL="http://43.143.135.49:3010"
TOKEN="your_jwt_token_here"
API_KEY="vibe-mcp-secret-key"

PASS=0
FAIL=0

test_endpoint() {
  local name="$1"
  local expected="$2"
  local cmd="$3"
  
  result=$(eval "$cmd")
  if [[ "$result" == *"$expected"* ]] || [[ "$expected" == "any" ]]; then
    echo "✅ PASS: $name"
    ((PASS++))
  else
    echo "❌ FAIL: $name"
    echo "   Expected: $expected"
    echo "   Got: $result"
    ((FAIL++))
  fi
}

# 健康检查
test_endpoint "Health Check" "OK" "curl -s $BASE_URL/health"

# 认证测试
test_endpoint "Login" "token" "curl -s -X POST $BASE_URL/api/auth/login -H 'Content-Type: application/json' -d '{\"email\":\"autoglm@test.com\",\"password\":\"test123456\"}'"

# 状态获取
test_endpoint "Get Status (JWT)" "taskCount" "curl -s $BASE_URL/api/status -H \"Authorization: Bearer $TOKEN\""

# 无认证应失败
test_endpoint "No Auth" "401" "curl -s -o /dev/null -w '%{http_code}' $BASE_URL/api/status"

echo ""
echo "========================================="
echo "测试结果: $PASS 通过, $FAIL 失败"
echo "========================================="
```

---

## 测试执行检查清单

- [ ] TC-001 ~ TC-006: 认证与任务CRUD
- [ ] TC-007: 任务同步
- [ ] TC-008: 任务删除
- [ ] TC-009: 历史记录查询
- [ ] TC-010: 阶段历史
- [ ] TC-011: 任务重置
- [ ] TC-012: JWT Token验证
- [ ] TC-013: API Key验证
- [ ] TC-014: 并发测试
- [ ] TC-015: 性能测试
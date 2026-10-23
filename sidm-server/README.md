# ☁️ SIDM Cloud Licensing & Update Server (Laravel)

This backend handles **Hardware ID (HWID) activation**, device limits, silent extractor engine updates, and payment gateway webhooks (bKash, Nagad, Paddle, LemonSqueezy).

---

## 🚀 Setup Instructions

1. Copy the contents of `sidm-server/` into your Laravel project or install a fresh Laravel app:
   ```bash
   composer create-project laravel/laravel sidm-cloud
   ```
2. Copy migrations, models, controllers, and routes from `sidm-server/` into your app.
3. Run migrations:
   ```bash
   php artisan migrate
   ```
4. Configure `.env`:
   ```env
   SIDM_WEBHOOK_SECRET=your_super_secret_webhook_token
   ```

---

## 📡 API Endpoints

### 1. Device Activation (`POST /api/v1/license/activate`)
Request:
```json
{
  "license_key": "SIDM-PRO-A1B2-C3D4-E5F6",
  "hwid": "SIDM-7FA1-889B-42C1-D990",
  "os": "windows",
  "app_version": "1.0.0"
}
```

### 2. Payment Gateway Webhook (`POST /api/v1/license/issue-webhook`)
Header: `X-SIDM-Secret: your_super_secret_webhook_token`
Request:
```json
{
  "email": "customer@gmail.com",
  "name": "Customer Name",
  "plan": "lifetime"
}
```
Response:
```json
{
  "success": true,
  "license_key": "SIDM-LIFE-78AE-9B21",
  "email": "customer@gmail.com"
}
```

### 3. Silent Engine Updates (`GET /api/v1/updates/check`)
Returns the latest `yt-dlp` sidecar hash & download URL so the SIDM desktop app automatically stays working even when YouTube updates their algorithms!

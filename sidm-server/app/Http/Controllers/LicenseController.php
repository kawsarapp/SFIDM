<?php

namespace App\Http\Controllers;

use App\Models\License;
use App\Models\LicenseActivation;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\Validator;

class LicenseController extends Controller {

    /**
     * Activate a license for a specific HWID
     */
    public function activate(Request $request): JsonResponse {
        $validator = Validator::make($request->all(), [
            'license_key' => 'required|string',
            'hwid'        => 'required|string',
            'os'          => 'nullable|string',
            'app_version' => 'nullable|string',
        ]);

        if ($validator->fails()) {
            return response()->json(['success' => false, 'error' => $validator->errors()->first()], 422);
        }

        $key = strtoupper(trim($request->input('license_key')));
        $hwid = strtoupper(trim($request->input('hwid')));

        $license = License::where('license_key', $key)->first();

        if (!$license) {
            return response()->json(['success' => false, 'error' => 'Invalid license key.'], 404);
        }

        if (!$license->is_active) {
            return response()->json(['success' => false, 'error' => 'This license key has been suspended.'], 403);
        }

        if ($license->expires_at && $license->expires_at->isPast()) {
            return response()->json(['success' => false, 'error' => 'This license key has expired.'], 403);
        }

        // Check existing activation for this HWID
        $existing = LicenseActivation::where('license_id', $license->id)
            ->where('hwid', $hwid)
            ->first();

        if ($existing) {
            $existing->update([
                'last_active_at' => now(),
                'ip_address'     => $request->ip(),
                'app_version'    => $request->input('app_version'),
            ]);
        } else {
            // Check device limit
            $currentDevices = $license->activations()->count();
            if ($currentDevices >= $license->max_devices) {
                return response()->json([
                    'success' => false,
                    'error'   => "Device limit reached. This license is already in use on {$license->max_devices} PC(s)."
                ], 403);
            }

            LicenseActivation::create([
                'license_id'     => $license->id,
                'hwid'           => $hwid,
                'os'             => $request->input('os', 'windows'),
                'app_version'    => $request->input('app_version', '1.0.0'),
                'ip_address'     => $request->ip(),
                'last_active_at' => now(),
            ]);
        }

        return response()->json([
            'success' => true,
            'data'    => [
                'license_key'    => $license->license_key,
                'plan'           => $license->plan,
                'customer_email' => $license->customer_email,
                'customer_name'  => $license->customer_name,
                'expires_at'     => $license->expires_at?->toIso8601String(),
                'hwid'           => $hwid,
                'is_pro'         => true,
            ]
        ]);
    }

    /**
     * Silent verification check
     */
    public function verify(Request $request): JsonResponse {
        $key = strtoupper(trim($request->input('license_key', '')));
        $hwid = strtoupper(trim($request->input('hwid', '')));

        $license = License::where('license_key', $key)
            ->where('is_active', true)
            ->first();

        if (!$license || ($license->expires_at && $license->expires_at->isPast())) {
            return response()->json(['valid' => false]);
        }

        $activation = LicenseActivation::where('license_id', $license->id)
            ->where('hwid', $hwid)
            ->first();

        if (!$activation) {
            return response()->json(['valid' => false]);
        }

        $activation->update(['last_active_at' => now()]);

        return response()->json([
            'valid' => true,
            'plan'  => $license->plan,
        ]);
    }

    /**
     * Webhook to automatically generate and issue a license after payment
     */
    public function issueAfterPayment(Request $request): JsonResponse {
        // Authenticate webhook secret
        $secret = $request->header('X-SIDM-Secret');
        if ($secret !== env('SIDM_WEBHOOK_SECRET', 'sidm_secret_change_me')) {
            return response()->json(['error' => 'Unauthorized'], 401);
        }

        $email = $request->input('email');
        $name = $request->input('name');
        $plan = $request->input('plan', 'pro_yearly'); // 'lifetime' or 'pro_yearly'

        $newKey = License::generateKey($plan);

        $license = License::create([
            'license_key'    => $newKey,
            'customer_email' => $email,
            'customer_name'  => $name,
            'plan'           => $plan,
            'max_devices'    => $plan === 'lifetime' ? 2 : 1,
            'is_active'      => true,
            'expires_at'     => $plan === 'lifetime' ? null : now()->addYear(),
        ]);

        return response()->json([
            'success'     => true,
            'license_key' => $license->license_key,
            'email'       => $license->customer_email,
        ]);
    }
}

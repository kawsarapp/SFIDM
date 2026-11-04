<?php

namespace App\Models;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\HasMany;

class License extends Model {
    protected $fillable = [
        'license_key',
        'customer_email',
        'customer_name',
        'plan',
        'max_devices',
        'is_active',
        'expires_at',
    ];

    protected $casts = [
        'is_active' => 'boolean',
        'expires_at' => 'datetime',
    ];

    public function activations(): HasMany {
        return $this->hasMany(LicenseActivation::class);
    }

    public static function generateKey(string $plan = 'pro'): string {
        $prefix = match ($plan) {
            'lifetime' => 'SIDM-LIFE',
            'standard' => 'SIDM-STD',
            default => 'SIDM-PRO',
        };
        $p1 = strtoupper(bin2hex(random_bytes(2)));
        $p2 = strtoupper(bin2hex(random_bytes(2)));
        $p3 = strtoupper(bin2hex(random_bytes(2)));
        return "{$prefix}-{$p1}-{$p2}-{$p3}";
    }
}

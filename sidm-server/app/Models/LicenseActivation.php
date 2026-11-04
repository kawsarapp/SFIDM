<?php

namespace App\Models;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\BelongsTo;

class LicenseActivation extends Model {
    protected $fillable = [
        'license_id',
        'hwid',
        'os',
        'app_version',
        'ip_address',
        'last_active_at',
    ];

    protected $casts = [
        'last_active_at' => 'datetime',
    ];

    public function license(): BelongsTo {
        return $this->belongsTo(License::class);
    }
}

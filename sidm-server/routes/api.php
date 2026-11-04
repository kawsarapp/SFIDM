<?php

use App\Http\Controllers\LicenseController;
use App\Http\Controllers\UpdateController;
use Illuminate\Support\Facades\Route;

Route::prefix('v1')->group(function () {
    // License verification and activation
    Route::post('/license/activate', [LicenseController::class, 'activate']);
    Route::post('/license/verify', [LicenseController::class, 'verify']);
    Route::post('/license/issue-webhook', [LicenseController::class, 'issueAfterPayment']);

    // Silent engine & app updates
    Route::get('/updates/check', [UpdateController::class, 'checkUpdate']);
});

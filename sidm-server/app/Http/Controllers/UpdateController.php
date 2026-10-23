<?php

namespace App\Http\Controllers;

use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;

class UpdateController extends Controller {

    /**
     * Check for SIDM App or Extractor Engine Updates
     */
    public function checkUpdate(Request $request): JsonResponse {
        $currentAppVersion = $request->query('app_version', '1.0.0');
        $currentEngineVersion = $request->query('engine_version', '2026.01.01');

        $latestApp = [
            'version'      => '1.0.0',
            'has_update'   => version_compare($currentAppVersion, '1.0.0', '<'),
            'release_notes'=> "Initial release of SIDM Next-Gen Hybrid Download Manager.",
            'download_url' => "https://sidm-downloads.example.com/sidm_setup_1.0.0.exe",
        ];

        // Silent extractor engine update (yt-dlp sidecar update)
        $latestEngine = [
            'version'      => '2026.10.20',
            'has_update'   => version_compare($currentEngineVersion, '2026.10.20', '<'),
            'download_url' => "https://sidm-downloads.example.com/engine/latest_yt_dlp.zip",
            'sha256'       => "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        ];

        return response()->json([
            'success' => true,
            'app'     => $latestApp,
            'engine'  => $latestEngine,
        ]);
    }
}

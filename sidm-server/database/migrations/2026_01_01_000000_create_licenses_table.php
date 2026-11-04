<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration {
    public function up(): void {
        Schema::create('licenses', function (Blueprint $table) {
            $table->id();
            $table->string('license_key')->unique();
            $table->string('customer_email')->nullable()->index();
            $table->string('customer_name')->nullable();
            $table->enum('plan', ['standard', 'pro_yearly', 'lifetime'])->default('pro_yearly');
            $table->integer('max_devices')->default(1);
            $table->boolean('is_active')->default(true);
            $table->timestamp('expires_at')->nullable();
            $table->timestamps();
        });

        Schema::create('license_activations', function (Blueprint $table) {
            $table->id();
            $table->foreignId('license_id')->constrained('licenses')->onDelete('cascade');
            $table->string('hwid'); // Machine Hardware ID (e.g. SIDM-XXXX-XXXX-XXXX-XXXX)
            $table->string('os')->nullable();
            $table->string('app_version')->nullable();
            $table->string('ip_address')->nullable();
            $table->timestamp('last_active_at')->nullable();
            $table->timestamps();

            $table->unique(['license_id', 'hwid']);
        });
    }

    public function down(): void {
        Schema::dropIfExists('license_activations');
        Schema::dropIfExists('licenses');
    }
};

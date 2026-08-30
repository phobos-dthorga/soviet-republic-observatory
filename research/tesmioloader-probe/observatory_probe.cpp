// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Republic Observatory contributors
// Optional same-process observation companion for TesmioLoader.
// This source writes only its own bounded telemetry file. It never writes game
// objects, save buffers, game files, Observatory databases, or the network.

#include "tesmio_plugin.h"

#define RVA_PERSON_VECTOR 0x9E75B8
#define RVA_WORLD_PTR 0x9941F0
#define OFF_DAY 0x590
#define OFF_YEAR 0x594
#define PERSON_SIZE 0x750
#define PERSON_CURRENT_BUILDING 0x20
#define PERSON_EDUCATION 0xA8
#define PERSON_AGE 0xD4
#define PERSON_STATUS 0xD8
#define PERSON_CLASS 0x71C
#define PERSON_MONEY_SPENT 0x734
#define TESTED_EXE_TIMESTAMP 0x6A3EB6ADu
#define TESTED_EXE_SIZE 10308608u
#define SYM_TERRAIN_RENDER "?Render@C3D_TERRAIN@@QEAAX_NPEAVC3D_CAMERA@@0HH@Z"
#define PROBE_FILE "republic-observatory-probe.jsonl"
#define MAX_SAMPLES 32
#define MAX_STATUS_VALUE 1.5f
#define MAX_MONEY_SPENT 1.0e12f

typedef void (*t_TerrainRender)(void*, bool, void*, void*, int, int);
static t_TerrainRender o_TerrainRender;
static HANDLE g_output = INVALID_HANDLE_VALUE;
static int g_enabled = 1;
static int g_samples = 16;
static int g_everyDays = 1;
static int g_maxRecords = 4096;
static int g_records;
static int g_sequence;
static int g_lastDay = -1;
static int g_lastYear = -1;
static BYTE** g_lastBegin;
static int g_lastCount;

static bool WriteLine(const char* line)
{
    if (g_output == INVALID_HANDLE_VALUE || g_records >= g_maxRecords) return false;
    TsmWrite(g_output, line, (int)strlen(line));
    TsmWrite(g_output, "\r\n", 2);
    FlushFileBuffers(g_output);
    g_records++;
    return true;
}

static unsigned ExeTimestamp(void)
{
    if (!ReadablePtr(g_exeBase, sizeof(IMAGE_DOS_HEADER))) return 0;
    const IMAGE_DOS_HEADER* dos = (const IMAGE_DOS_HEADER*)g_exeBase;
    if (dos->e_magic != IMAGE_DOS_SIGNATURE || dos->e_lfanew <= 0) return 0;
    const BYTE* ntAddress = g_exeBase + dos->e_lfanew;
    if (!ReadablePtr(ntAddress, sizeof(IMAGE_NT_HEADERS64))) return 0;
    const IMAGE_NT_HEADERS64* nt = (const IMAGE_NT_HEADERS64*)ntAddress;
    if (nt->Signature != IMAGE_NT_SIGNATURE) return 0;
    return nt->FileHeader.TimeDateStamp;
}

static BYTE* World(void)
{
    BYTE** slot = (BYTE**)(g_exeBase + RVA_WORLD_PTR);
    if (!ReadablePtr(slot, sizeof(void*))) return NULL;
    BYTE* world = *slot;
    return world && ReadablePtr(world, 0x600) ? world : NULL;
}

static int People(BYTE*** outBegin)
{
    BYTE*** vector = (BYTE***)(g_exeBase + RVA_PERSON_VECTOR);
    if (!ReadablePtr(vector, 16)) return 0;
    BYTE** begin = (BYTE**)vector[0];
    BYTE** end = (BYTE**)vector[1];
    if (!begin || end < begin) return 0;
    size_t bytes = (size_t)((BYTE*)end - (BYTE*)begin);
    if (bytes % sizeof(void*)) return 0;
    size_t count = bytes / sizeof(void*);
    if (count == 0 || count > 500000) return 0;
    *outBegin = begin;
    return (int)count;
}

static bool Finite(float value)
{
    return value == value && value < 3.0e38f && value > -3.0e38f;
}

static bool SanePerson(BYTE* person)
{
    if (!person || !ReadablePtr(person, PERSON_SIZE)) return false;
    float age = *(const float*)(person + PERSON_AGE);
    float education = *(const float*)(person + PERSON_EDUCATION);
    int personClass = *(const int*)(person + PERSON_CLASS);
    if (!Finite(age) || age < 0.0f || age > 200.0f) return false;
    if (!Finite(education) || education < 0.0f || education > 3.0f) return false;
    if (personClass < 0 || personClass > 2) return false;
    for (int i = 0; i < 11; i++) {
        float status = *(const float*)(person + PERSON_STATUS + i * 4);
        if (!Finite(status) || status < 0.0f || status > MAX_STATUS_VALUE) return false;
    }
    float moneySpent = *(const float*)(person + PERSON_MONEY_SPENT);
    return Finite(moneySpent) && moneySpent >= 0.0f && moneySpent <= MAX_MONEY_SPENT;
}

static void WriteSession(void)
{
    char line[2048];
    _snprintf_s(
        line, sizeof(line), _TRUNCATE,
        "{\"schema_version\":1,\"record_type\":\"session\",\"probe_id\":\"org.republic-observatory.tesmio-readonly\",\"probe_version\":\"0.1.0\",\"mode\":\"read_only\",\"loader_api_version\":%u,\"target_game_version\":\"1.1.1.9\",\"executable_timestamp\":%u,\"executable_size\":%llu,\"person_size\":%u,\"person_vector_rva\":\"0x9E75B8\",\"writes_game_state\":false,\"writes_save_data\":false,\"writes_observatory_databases\":false,\"network_access\":false}",
        TSM_API_VERSION, ExeTimestamp(), (unsigned long long)g_exeSize, PERSON_SIZE);
    WriteLine(line);
}

static void WriteSample(BYTE* person, int sequence, int sampleIndex, int vectorIndex, int year, int day)
{
    const float* status = (const float*)(person + PERSON_STATUS);
    char line[4096];
    _snprintf_s(
        line, sizeof(line), _TRUNCATE,
        "{\"schema_version\":1,\"record_type\":\"person_sample\",\"sequence\":%d,\"sample_index\":%d,\"vector_index\":%d,\"year\":%d,\"day\":%d,\"current_building_present\":%s,\"age_years\":%.9g,\"education_level\":%.9g,\"status_happiness\":%.9g,\"status_food\":%.9g,\"status_health\":%.9g,\"status_soviet\":%.9g,\"status_alcohol\":%.9g,\"status_culture\":%.9g,\"status_sport\":%.9g,\"status_religion\":%.9g,\"status_clothing\":%.9g,\"status_electronics\":%.9g,\"status_crime\":%.9g,\"citizen_class\":%d,\"money_spent\":%.9g}",
        sequence, sampleIndex, vectorIndex, year, day,
        *(BYTE**)(person + PERSON_CURRENT_BUILDING) ? "true" : "false",
        *(float*)(person + PERSON_AGE), *(float*)(person + PERSON_EDUCATION),
        status[0], status[1], status[2], status[3], status[4], status[5],
        status[6], status[7], status[8], status[9], status[10],
        *(int*)(person + PERSON_CLASS), *(float*)(person + PERSON_MONEY_SPENT));
    WriteLine(line);
}

static void Snapshot(BYTE** begin, int count, int year, int day)
{
    int wanted = g_samples < count ? g_samples : count;
    if (wanted < 1 || g_records + wanted + 1 > g_maxRecords) {
        g_enabled = 0;
        Logf("observatory_probe stopped at its bounded record limit");
        return;
    }
    int step = count / wanted;
    if (step < 1) step = 1;
    int indices[MAX_SAMPLES];
    BYTE* people[MAX_SAMPLES];
    int valid = 0;
    for (int i = 0; i < wanted; i++) {
        int index = i * step;
        if (!ReadablePtr(begin + index, sizeof(void*))) continue;
        BYTE* person = begin[index];
        if (!SanePerson(person)) continue;
        indices[valid] = index;
        people[valid++] = person;
    }
    int sequence = ++g_sequence;
    char line[512];
    _snprintf_s(line, sizeof(line), _TRUNCATE,
        "{\"schema_version\":1,\"record_type\":\"snapshot\",\"sequence\":%d,\"year\":%d,\"day\":%d,\"population_count\":%d,\"sample_count\":%d}",
        sequence, year, day, count, valid);
    WriteLine(line);
    for (int i = 0; i < valid; i++)
        WriteSample(people[i], sequence, i, indices[i], year, day);
}

static void Tick(void)
{
    BYTE* world = World();
    if (!world) return;
    int day = *(const int*)(world + OFF_DAY);
    int year = *(const int*)(world + OFF_YEAR);
    if (day < 0 || day > 365 || year < 1900 || year > 10000) return;
    BYTE** begin = NULL;
    int count = People(&begin);
    if (count <= 0) return;
    bool worldChanged = begin != g_lastBegin || count != g_lastCount;
    bool dateChanged = day != g_lastDay || year != g_lastYear;
    int absoluteDay = year * 365 + day;
    int lastAbsoluteDay = g_lastYear * 365 + g_lastDay;
    bool intervalPassed = g_lastDay < 0 || abs(absoluteDay - lastAbsoluteDay) >= g_everyDays;
    if (!worldChanged && (!dateChanged || !intervalPassed)) return;
    g_lastBegin = begin;
    g_lastCount = count;
    g_lastDay = day;
    g_lastYear = year;
    Snapshot(begin, count, year, day);
}

static void h_TerrainRender(void* self, bool a, void* cam1, void* cam2, int b, int c)
{
    o_TerrainRender(self, a, cam1, cam2, b, c);
    if (!g_enabled) return;
    __try { Tick(); }
    __except (FaultFilter("Republic Observatory read-only probe", GetExceptionInformation())) {
        g_enabled = 0;
        Logf("observatory_probe disabled after a guarded read fault");
    }
}

static void ReadSettings(void)
{
    const char* ini = "plugins\\observatory_probe.ini";
    g_enabled = H->configInt(ini, "observatory_probe", "enabled", g_enabled);
    g_samples = H->configInt(ini, "observatory_probe", "samples", g_samples);
    g_everyDays = H->configInt(ini, "observatory_probe", "every_days", g_everyDays);
    g_maxRecords = H->configInt(ini, "observatory_probe", "max_records", g_maxRecords);
    if (g_samples < 1) g_samples = 1;
    if (g_samples > MAX_SAMPLES) g_samples = MAX_SAMPLES;
    if (g_everyDays < 1) g_everyDays = 1;
    if (g_everyDays > 365) g_everyDays = 365;
    if (g_maxRecords < 257) g_maxRecords = 257;
    if (g_maxRecords > 8192) g_maxRecords = 8192;
}

extern "C" __declspec(dllexport) unsigned TsmPluginApiVersion(void) { return TSM_API_VERSION; }

extern "C" __declspec(dllexport) int TsmPluginInit(const TsmHost* host, TsmPluginInfo* info)
{
    TsmBind(host);
    info->name = "Republic Observatory read-only probe";
    info->version = "0.1.0";
    ReadSettings();
    unsigned timestamp = ExeTimestamp();
    if (!g_enabled) return 1;
    if (timestamp != TESTED_EXE_TIMESTAMP || g_exeSize != TESTED_EXE_SIZE) {
        Logf("observatory_probe refused unsupported executable (timestamp=%08X size=%llu)",
             timestamp, (unsigned long long)g_exeSize);
        return 1;
    }
    g_output = TsmOpenLog(PROBE_FILE);
    if (g_output == INVALID_HANDLE_VALUE) {
        Logf("observatory_probe could not create its fixed telemetry file");
        return 1;
    }
    WriteSession();
    return 0;
}

extern "C" __declspec(dllexport) int TsmPluginStart(void)
{
    if (!g_enabled || g_output == INVALID_HANDLE_VALUE) return 1;
    if (!PatchIat(g_exe, DLL_ENGINE, SYM_TERRAIN_RENDER, (void*)h_TerrainRender,
                  (void**)&o_TerrainRender, "C3D_TERRAIN::Render")) {
        Logf("observatory_probe found no chainable render import; no observation started");
        return 1;
    }
    Logf("observatory_probe armed: sampled reads only; no game or save writes");
    return 0;
}

BOOL APIENTRY DllMain(HMODULE, DWORD, LPVOID) { return TRUE; }

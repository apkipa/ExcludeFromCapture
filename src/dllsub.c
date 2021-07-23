#include <Windows.h>

// typedef struct tagPipeDllParams {
//     HWND target_hwnd;
//     DWORD affinity;
// } PipeDllParams;
typedef struct tagPipeDllParams {
    DWORD target_hwnd;
    DWORD affinity;
} PipeDllParams;

BOOL IsNewDataAvailableFromPipe(HANDLE hPipe) {
    DWORD dwAvail;
    if (PeekNamedPipe(hPipe, NULL, 0, NULL, &dwAvail, NULL) == FALSE) {
        return FALSE;
    }
    return dwAvail > 0;
}

BOOL GetDllParamsFromPipe(PipeDllParams *pParam) {
    DWORD dwRead, dwProcId = 0;
    HANDLE hPipe;

    hPipe = CreateFileW(
        L"\\\\.\\pipe\\excludefromcapture_pipedlldata",
        GENERIC_READ,
        0,
        NULL,
        OPEN_EXISTING,
        FILE_ATTRIBUTE_NORMAL,
        NULL
    );
    if (hPipe == INVALID_HANDLE_VALUE) {
        return FALSE;
    }
    for (int i = 0; i < 30; i++) {
        Sleep(100);
        if (IsNewDataAvailableFromPipe(hPipe)) {
            goto WAIT_SUCCEEDED;
        }
    }
    CloseHandle(hPipe);
    return FALSE;

WAIT_SUCCEEDED:
    if (ReadFile(hPipe, pParam, sizeof(PipeDllParams), &dwRead, NULL) == FALSE) {
        CloseHandle(hPipe);
        return FALSE;
    }
    CloseHandle(hPipe);
    if (dwRead != sizeof(PipeDllParams)) {
        return FALSE;
    }

    // Check whether target_hwnd corresponds to current process
    GetWindowThreadProcessId((HWND)pParam->target_hwnd, &dwProcId);
    if (dwProcId != GetCurrentProcessId()) {
        return FALSE;
    }

    return TRUE;
}

DWORD WINAPI ThreadProc(LPVOID lpParameter) {
    PipeDllParams params;

    if (GetDllParamsFromPipe(&params) == FALSE) {
        MessageBoxW(NULL, L"Failed getting params", NULL, MB_ICONERROR);
        goto DLL_SUICIDE;
    }

    if (SetWindowDisplayAffinity((HWND)params.target_hwnd, params.affinity) == FALSE) {
        MessageBoxW(NULL, L"Failed setting display affinity", NULL, MB_ICONERROR);
        goto DLL_SUICIDE;
    }

DLL_SUICIDE:
    FreeLibraryAndExitThread((HMODULE)lpParameter, 0);
}

BOOL WINAPI DllMain(HINSTANCE hinstDLL, DWORD fdwReason, LPVOID lpReserved) {
    switch (fdwReason) {
    case DLL_PROCESS_ATTACH:
        CloseHandle(CreateThread(NULL, 0, ThreadProc, hinstDLL, 0, NULL));
        break;
    case DLL_PROCESS_DETACH:
        break;
    }

    return TRUE;
}

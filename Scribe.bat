@echo off
setlocal enabledelayedexpansion

:: Set constants
set "fs=44100"
set "duration=4500"
set "output_directory=C:\users\bhara\documents\transcriber\recordings"
set "base_filename=recording"

:MainMenu
cls
echo Main Menu:
echo 1. Transcribe an existing mp3 file
echo 2. Start new recording
echo 3. Exit
echo.
set /p userChoice=Enter your choice (1/2/3): 
if "%userChoice%"=="1" goto TranscribeExisting
if "%userChoice%"=="2" goto StartRecording
if "%userChoice%"=="3" goto ExitScript
echo Invalid choice. Please try again.
pause
goto MainMenu

:TranscribeExisting
set /p mp3Path=Enter the path of the MP3 file to transcribe: 
if not exist "%mp3Path%" (
    echo File not found. Please try again.
    pause
    goto TranscribeExisting
)
python transcribe.py "%mp3Path%"
pause
goto MainMenu

:StartRecording
echo Press 'y' to start recording...
choice /c y /n 
if errorlevel 2 goto MainMenu

:: Get the current date and time in the format ddmmyyyy_HHMMSS
for /f "tokens=1-3 delims=/: " %%a in ("%date% %time%") do (
    set "formatted_datetime=%%a%%b%%c_%%d%%e%%f"
)

set "output_filename=%base_filename%!formatted_datetime!.wav"
echo Recording started...press q to stop recording

:: Create the output directory if it doesn't exist
if not exist "%output_directory%" (
    mkdir "%output_directory%"
)

ffmpeg -loglevel panic -f dshow -i audio="Microphone Array (Realtek(R) Audio)" -t %duration% -acodec pcm_s16le -ar %fs% "%output_directory%\%output_filename%"
echo Recording stopped.
goto convertToMp3

:convertToMp3
set "mp3_output_filename=%base_filename%!formatted_datetime!.mp3"

:: Check if the WAV file exists
if exist "%output_directory%\%output_filename%" (
    :: Convert to MP3 and save to the specified location
    ffmpeg -loglevel panic -i "%output_directory%\%output_filename%" -codec:a libmp3lame -q:a 2 "%output_directory%\%mp3_output_filename%"
    echo MP3 conversion completed.
    
    :: Pass the recorded MP3 to main.py and generate a transcript
    python transcribe.py "%output_directory%\%mp3_output_filename%"
    
    :: Delete the WAV file after conversion to MP3
    del "%output_directory%\%output_filename%"
) else (
    echo No recording found. Skipping MP3 conversion.
)
pause
goto MainMenu

:ExitScript
exit /b 0

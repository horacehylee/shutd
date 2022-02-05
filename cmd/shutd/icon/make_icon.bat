@ECHO OFF

IF "%GOPATH%"=="" GOTO NOGO
IF NOT EXIST %GOPATH%\bin\crossicon.exe GOTO INSTALL
:POSTINSTALL
IF "%1"=="" GOTO NOICO
IF NOT EXIST %1 GOTO BADFILE
ECHO Creating iconwin.go
%GOPATH%\bin\crossicon -i %1 --bytes -o icon -p icon
GOTO DONE

:CREATEFAIL
ECHO Unable to create output file
GOTO DONE

:INSTALL
ECHO Installing crossicon...
go get github.com/cdujeu/crossicon
IF ERRORLEVEL 1 GOTO GETFAIL
GOTO POSTINSTALL

:GETFAIL
ECHO Failure running go get github.com/cdujeu/crossicon.  Ensure that go and git are in PATH
GOTO DONE

:NOGO
ECHO GOPATH environment variable not set
GOTO DONE

:NOICO
ECHO Please specify a .ico file
GOTO DONE

:BADFILE
ECHO %1 is not a valid file
GOTO DONE

:DONE
package solverforge

import "fmt"

// ErrorCode represents the type of error
type ErrorCode uint32

const (
	ErrorCodeUnknown       ErrorCode = 0
	ErrorCodeSerialization ErrorCode = 1
	ErrorCodeHTTP          ErrorCode = 2
	ErrorCodeSolver        ErrorCode = 3
	ErrorCodeWasm          ErrorCode = 4
	ErrorCodeBridge        ErrorCode = 5
	ErrorCodeValidation    ErrorCode = 6
	ErrorCodeConfiguration ErrorCode = 7
	ErrorCodeService       ErrorCode = 8
	ErrorCodeIO            ErrorCode = 9
)

// String returns the string representation of the error code
func (c ErrorCode) String() string {
	switch c {
	case ErrorCodeSerialization:
		return "Serialization"
	case ErrorCodeHTTP:
		return "HTTP"
	case ErrorCodeSolver:
		return "Solver"
	case ErrorCodeWasm:
		return "WASM"
	case ErrorCodeBridge:
		return "Bridge"
	case ErrorCodeValidation:
		return "Validation"
	case ErrorCodeConfiguration:
		return "Configuration"
	case ErrorCodeService:
		return "Service"
	case ErrorCodeIO:
		return "IO"
	default:
		return "Unknown"
	}
}

// SolverForgeError represents an error from the SolverForge library
type SolverForgeError struct {
	Code    ErrorCode
	Message string
}

// Error implements the error interface
func (e *SolverForgeError) Error() string {
	return fmt.Sprintf("[%s] %s", e.Code.String(), e.Message)
}

// NewError creates a new SolverForgeError
func NewError(code ErrorCode, message string) *SolverForgeError {
	return &SolverForgeError{
		Code:    code,
		Message: message,
	}
}

package shutdown

import (
	"errors"
	"fmt"
	"time"

	"github.com/gen2brain/dlgs"
)

var questionTimeoutError = errors.New("question is timed out")

func question(title, text string, timeout time.Duration) (bool, error) {
	type result struct {
		yes bool
		err error
	}
	chanResult := make(chan result, 1)
	go func() {
		yes, err := dlgs.Question(title, text, true)
		chanResult <- result{yes, err}
	}()
	select {
	case res := <-chanResult:
		return res.yes, res.err
	case <-time.After(timeout):
		fmt.Println("timed out")
		return false, questionTimeoutError
	}
}

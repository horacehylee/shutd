//go:build !windows

package shutd

import (
	"context"

	"github.com/gen2brain/dlgs"
)

func question(ctx context.Context, title, text string) (bool, error) {
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
	case <-ctx.Done():
		return false, ctx.Err()
	}
}

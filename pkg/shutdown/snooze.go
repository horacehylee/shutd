package shutdown

import (
	"context"
	"errors"
	"fmt"
	"time"

	"github.com/gen2brain/dlgs"
)

func (s *scheduler) newSnoozeNotificationTask() func() {
	return func() {
		s.logger.Info("==========================")
		s.logger.Info("Snooze notification")
		s.logger.Info("==========================")

		title := fmt.Sprintf("Shutd - Scheduled Shutdown at %v", s.shutdownJob.ScheduledAtTime())
		text := fmt.Sprintf("About to shutdown in %v minutes, wanted to snooze for %v minutes?", s.config.Notification.Before, s.config.SnoozeInterval)
		ctx, cancel := context.WithTimeout(context.Background(), time.Duration(s.config.Notification.Duration)*time.Minute)
		defer cancel()

		yes, err := question(ctx, title, text)
		if err != nil && !errors.Is(err, context.Canceled) && !errors.Is(err, context.DeadlineExceeded) {
			s.logger.Errorf("failed to display snooze notification: %v", err)
			return
		}
		s.logger.Infof("snooze is required: %v", yes)
		if yes {
			err := s.Snooze()
			if err != nil {
				s.logger.Errorf("failed to snooze: %v", err)
			}
		}
	}
}

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

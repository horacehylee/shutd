package shutdown

import (
	"fmt"
	"os/exec"
)

func (s *scheduler) newShutdownTask() func() {
	return func() {
		s.logger.Info("==========================")
		s.logger.Info("Shutdown")
		s.logger.Info("==========================")
		shutdown()
	}
}

func shutdown() (err error) {
	if err := exec.Command("cmd", "/C", "shutdown", "/t", "0", "/s").Run(); err != nil {
		return fmt.Errorf("failed to initiate shutdown: %w", err)
	}
	return nil
}
